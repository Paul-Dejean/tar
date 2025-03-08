use std::{
    collections::HashMap,
    fs::{self, File},
    io::{stdin, Read, Write},
    os::unix::fs::MetadataExt,
    str,
    time::UNIX_EPOCH,
};

use clap::Parser;
use std::os::unix::fs::PermissionsExt;
use users::get_group_by_gid;
use users::get_user_by_uid;

#[derive(Parser, Debug)]
#[command(name = "ctar")]
#[command(author = "Paul Dejean <pauldejeandev@gmail.com>")]
#[command(version = "1.0")]
#[command(about = "A copy of unix command line tool tar", long_about = None)]
pub struct Args {
    #[arg(short = 't', long = "list", group = "mode")]
    list: bool,

    #[arg(short = 'x', long = "extract", group = "mode")]
    extract: bool,

    #[arg(short = 'f', long = "file")]
    archive_name: Option<String>,

    #[arg(short = 'c', long = "create", group = "mode")]
    create: bool,

    entries: Option<Vec<String>>,
}

pub fn execute_command(args: &Args) -> i32 {
    match launch(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    }
}

fn launch(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    if !args.list && !args.extract && !args.create {
        return Err("No mode specified".into());
    }

    if args.create {
        if let None = args.entries {
            return Err("No file or directory specified".into());
        }
        if let None = args.archive_name {
            return Err("No archive name specified".into());
        }
        let entries = args.entries.as_ref().unwrap();
        let paths = entries.iter().map(String::as_str).collect();
        create_tarball(paths, args.archive_name.as_ref().unwrap());
        return Ok(());
    }
    let data = args
        .archive_name
        .as_ref()
        .map(|value| read_tar_achive(value))
        .unwrap_or_else(|| read_from_stdin())?;

    println!("{:?}", data);

    let blocks: Vec<[u8; 512]> = data
        .chunks_exact(512)
        .map(|chunk| chunk.try_into().expect("Invalid chunk size"))
        .collect();

    if args.list {
        let file_names = get_file_names(blocks)?;
        for file_name in file_names {
            println!("{}", file_name);
        }
    } else if args.extract {
        let files = extract_files(blocks)?;
        println!("{:?}", files);
        for (file_name, content) in files {
            fs::write(&file_name, content)?;
        }
    }
    return Ok(());
}

fn extract_files(
    blocks: Vec<[u8; 512]>,
) -> Result<HashMap<String, Vec<u8>>, Box<dyn std::error::Error>> {
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut current_file: Option<String> = None;
    let mut nb_bytes = 0;

    for block in blocks {
        if block.iter().all(|&x| x == 0) {
            if let Some(ref file_name) = current_file {
                truncate_file_content(file_name, &mut files, nb_bytes)?;
            }
        } else if is_valid_header(&block) {
            if let Some(ref file_name) = current_file {
                truncate_file_content(file_name, &mut files, nb_bytes)?;
            }

            let (file_name, content_size) = extract_file_info(&block)?;
            current_file = Some(file_name);
            nb_bytes = content_size;
            files.insert(current_file.clone().expect("file not found"), vec![]);
        } else {
            let file_name = current_file.as_ref().ok_or("Error: No file name found")?;
            let content = files
                .get_mut(file_name.as_str())
                .ok_or("File content not found")?;
            content.extend_from_slice(&block);
        }
    }
    return Ok(files);
}

fn truncate_file_content(
    file_name: &str,
    files: &mut HashMap<String, Vec<u8>>,
    nb_bytes: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = files.get_mut(file_name).ok_or("File content not found")?;
    if content.len() < nb_bytes {
        eprintln!("Error: Incomplete file");
        return Err("Error extracting file content".into());
    }
    content.truncate(nb_bytes);

    Ok(())
}

fn get_file_names(blocks: Vec<[u8; 512]>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut file_names = Vec::new();
    for block in blocks {
        if is_valid_header(&block) {
            let file_name = extract_file_name(&block)?;
            file_names.push(file_name);
        }
    }
    return Ok(file_names);
}

fn extract_file_info(
    header_block: &[u8; 512],
) -> Result<(String, usize), Box<dyn std::error::Error>> {
    let file_name = extract_file_name(header_block)?;
    let content_size = extract_content_size(header_block);
    return Ok((file_name, content_size));
}

fn extract_file_name(header_block: &[u8; 512]) -> Result<String, Box<dyn std::error::Error>> {
    let file_name_field = &header_block[0..100];
    let file_name = match str::from_utf8(file_name_field) {
        Ok(s) => s
            .trim_matches(|c: char| c == '\0' || c.is_whitespace())
            .trim(),
        Err(_) => {
            return Err("Error extracting file name".into());
        }
    };
    return Ok(file_name.to_string());
}

fn extract_content_size(header_block: &[u8; 512]) -> usize {
    let size_field = &header_block[124..136];
    let size_str = match str::from_utf8(size_field) {
        Ok(s) => s
            .trim_matches(|c: char| c == '\0' || c.is_whitespace())
            .trim(),
        Err(_) => {
            return 0;
        }
    };

    let size = match usize::from_str_radix(size_str, 8) {
        Ok(num) => num,
        Err(_) => {
            return 0;
        }
    };

    return size;
}

fn is_valid_header(block: &[u8; 512]) -> bool {
    let checksum_field = &block[148..156];

    let checksum_str = match str::from_utf8(checksum_field) {
        Ok(s) => s
            .trim_matches(|c: char| c == '\0' || c.is_whitespace())
            .trim(),
        Err(_) => {
            return false;
        }
    };

    if checksum_str.is_empty() {
        return false;
    }

    let stored_checksum = match u32::from_str_radix(checksum_str, 8) {
        Ok(num) => num,
        Err(_) => {
            return false;
        }
    };

    let sum = calculate_block_checksum(block);

    return sum == stored_checksum;
}

fn calculate_block_checksum(block: &[u8; 512]) -> u32 {
    let mut sum = 0;
    for i in 0..148 {
        sum += block[i] as u32;
    }
    sum += 32 * 8;
    for i in 156..512 {
        sum += block[i] as u32;
    }
    return sum;
}

fn read_tar_achive(file: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if !file.ends_with(".tar") {
        return Err("Invalid file extension, expected .tar".into());
    }
    let data = fs::read(file)?;
    return Ok(data);
}

fn read_from_stdin() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut content = Vec::new();
    stdin().lock().read_to_end(&mut content)?;
    Ok(content)
}

fn create_tarball(file_paths: Vec<&str>, tarball_name: &str) {
    let mut tarball = Vec::new();
    for file_path in file_paths {
        let header = create_header(file_path).expect("Error creating header");
        let file = fs::read(file_path).expect("Error reading file");
        let header_block = create_header_block(&header);
        let content_blocks = file_to_blocks(&file);
        tarball.push(header_block);
        tarball.extend(content_blocks);
    }
    tarball.push([0; 512]);
    tarball.push([0; 512]);
    let mut file = File::create(tarball_name).expect("Error creating tarball");
    for block in tarball {
        file.write_all(&block).expect("Error writing block");
    }
}

#[derive(Debug)]
struct TarHeader {
    file_name: String, // [0..100]
    mode: u32,         // [100..108]
    uid: u32,          // [108..116]
    gid: u32,          // [116..124]
    file_size: u64,    // [124..136]
    mtime: u64,        // [136..148]
    typeflag: u8,      // [156]
    link_name: String, // [157..257]
    magic: String,     // [257..263]
    version: String,   // [263..265]
    uname: String,     // [265..297]
    gname: String,     // [297..329]
    devmajor: u32,     // [329..337]
    devminor: u32,     // [337..345]
    prefix: String,    // [345..500]
                       // [500..512] is padding
}

fn create_header(file_name: &str) -> Result<TarHeader, Box<dyn std::error::Error>> {
    let meta = fs::metadata(file_name)?;
    let perm = meta.permissions().mode() & 0o777;
    let mode = perm;
    println!("mode: {}", mode);
    let uid = meta.uid();
    let gid = meta.gid();
    let file_size = meta.len();
    let mtime = meta
        .modified()?
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let typeflag = 0;
    let link_name = "";
    let magic = "ustar";
    let version = "00";
    let uname = get_user_by_uid(uid).map(|user| user.name().to_string_lossy().into_owned());
    let gname = get_group_by_gid(gid).map(|group| group.name().to_string_lossy().into_owned());
    let devmajor = 0;
    let devminor = 0;
    let prefix = "";

    let header = TarHeader {
        file_name: file_name.to_string(),
        mode,
        uid,
        gid,
        file_size,
        mtime,
        typeflag,
        link_name: link_name.to_string(),
        magic: magic.to_string(),
        version: version.to_string(),
        uname: uname.unwrap_or_default(),
        gname: gname.unwrap_or_default(),
        devmajor,
        devminor,
        prefix: prefix.to_string(),
    };
    return Ok(header);
}

fn create_header_block(header: &TarHeader) -> [u8; 512] {
    println!("{:?}", header);
    let mut block = [0; 512];
    insert_in_block(&mut block, &header.file_name, 0, 100);
    insert_in_block(
        &mut block,
        &format_octal(header.mode as u64, 7, " "),
        100,
        108,
    );
    insert_in_block(
        &mut block,
        &format_octal(header.uid as u64, 7, " "),
        108,
        116,
    );
    insert_in_block(
        &mut block,
        &format_octal(header.gid as u64, 7, " "),
        116,
        124,
    );
    insert_in_block(
        &mut block,
        &format_octal(header.file_size as u64, 12, " "),
        124,
        136,
    );
    insert_in_block(&mut block, &format_octal(header.mtime, 12, " "), 136, 148);
    insert_in_block(
        &mut block,
        &format_octal(header.typeflag as u64, 1, " "),
        156,
        157,
    );
    insert_in_block(&mut block, &header.link_name, 157, 257);
    insert_in_block(&mut block, &header.magic, 257, 263);
    insert_in_block(&mut block, &header.version, 263, 265);
    insert_in_block(&mut block, &header.uname, 265, 297);
    insert_in_block(&mut block, &header.gname, 297, 329);
    insert_in_block(
        &mut block,
        &format_octal(header.devmajor as u64, 7, " "),
        329,
        337,
    );
    insert_in_block(
        &mut block,
        &format_octal(header.devminor as u64, 7, " "),
        337,
        345,
    );
    insert_in_block(&mut block, &header.prefix, 345, 500);
    let checksum = calculate_block_checksum(&block);
    insert_in_block(
        &mut block,
        &(format_octal(checksum as u64, 7, "\0") + " "),
        148,
        156,
    );
    return block;
}

fn file_to_blocks(file: &[u8]) -> Vec<[u8; 512]> {
    let blocks = file
        .chunks(512)
        .map(|chunk| {
            let mut block = [0; 512];
            for (i, byte) in chunk.iter().enumerate() {
                block[i] = *byte;
            }
            block
        })
        .collect();

    return blocks;
}

fn insert_in_block(block: &mut [u8; 512], value: &str, start: usize, end: usize) {
    let bytes = value.as_bytes();
    println!("{value} {}", end - start);

    let len = bytes.len();
    println!("{:?}", bytes);

    if len > end - start {
        panic!("Value is too long");
    }
    for i in 0..len {
        block[start + i] = bytes[i];
    }
}

fn format_octal(value: u64, width: usize, terminator: &str) -> String {
    if width == 1 {
        return format!("{:o}", value);
    }
    return format!("{:0width$o}", value, width = width - 1) + terminator;
}
