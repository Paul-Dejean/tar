use std::{
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
    #[arg(short = 't', long = "list")]
    list: bool,

    #[arg(short = 'f', long = "file")]
    file: Option<String>,
}

pub fn execute_command(args: &Args) -> i32 {
    let data = match args
        .file
        .as_ref()
        .map(|value| read_tar_achive(value))
        .unwrap_or_else(|| read_from_stdin())
    {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    let blocks: Vec<[u8; 512]> = data
        .chunks_exact(512)
        .map(|chunk| chunk.try_into().expect("Invalid chunk size"))
        .collect();

    for block in blocks {
        if is_valid_header(&block) {
            let file_name = match extract_file_name(&block) {
                Ok(name) => name,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return 1;
                }
            };
            println!("{}", file_name);
        }
    }
    return 0;
}

fn extract_file_name(header_block: &[u8; 512]) -> Result<String, std::io::Error> {
    let file_name_field = &header_block[0..100];
    let file_name = match str::from_utf8(file_name_field) {
        Ok(s) => s
            .trim_matches(|c: char| c == '\0' || c.is_whitespace())
            .trim(),
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid file name",
            ));
        }
    };
    return Ok(file_name.to_string());
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
        Err(e) => {
            eprintln!("Error parsing checksum field as octal: {:?}", e);
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

pub fn read_from_stdin() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut content = Vec::new();
    stdin().lock().read_to_end(&mut content)?;
    Ok(content)
}
