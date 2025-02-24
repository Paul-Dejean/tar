use clap::Parser;
use ctar::{execute_command, Args};
use std::process;

fn main() {
    let args = Args::parse();
    let error_code = execute_command(&args);
    process::exit(error_code);
}
