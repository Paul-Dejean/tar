use std::{
    collections::HashMap,
    fs,
    io::{stdin, Read},
    str,
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ctar")]
#[command(author = "Ebooth <pauldejeandev@gmail.com>")]
#[command(version = "1.0")]
#[command(about = "A copy of unix command line tool tar", long_about = None)]
pub struct Args {
    #[arg(short = 't', long = "list", group = "mode")]
    pub list: bool,

    #[arg(short = 'x', long = "extract", group = "mode")]
    pub extract: bool,

    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,
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
    if !args.list && !args.extract {
        return Err("No mode specified".into());
    }
    let data = args
        .file
        .as_ref()
        .map(|value| read_tar_achive(value))
        .unwrap_or_else(|| read_from_stdin())?;

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
