use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::io::Write;
use std::process::Command as ProcessCommand;
use tempfile::{Builder, NamedTempFile, TempDir};

fn create_temp_dir() -> TempDir {
    let current_dir = env::current_dir().expect("error getting current directory");
    return Builder::new().tempdir_in(&current_dir).unwrap();
}

fn create_temp_file(dir: &TempDir, content: &str) -> NamedTempFile {
    let mut temp_file = Builder::new().tempfile_in(dir).unwrap();
    write!(temp_file, "{}", content).unwrap();
    return temp_file;
}

fn get_relative_path(file: &NamedTempFile) -> String {
    let current_dir = env::current_dir().expect("error getting current directory");
    return file
        .path()
        .strip_prefix(&current_dir)
        .expect("error getting relative path")
        .to_str()
        .expect("error converting path to str")
        .to_string();
}

#[test]
fn should_decode_content_of_tar_from_stdin() {
    let dir = create_temp_dir();
    let content = "Hello World";
    let file = create_temp_file(&dir, content);
    let file_path = &get_relative_path(&file);
    let archive_name = &format!("{}/archive.tar", { dir.path().to_str().unwrap() });

    ProcessCommand::new("tar")
        .args(["-cf", archive_name, file_path])
        .output()
        .expect("error creating tar archive");

    let archive_bytes = std::fs::read(&archive_name).expect("error reading tar archive from disk");

    let mut cmd = Command::cargo_bin("ctar").expect("error getting ctar binary");
    cmd.args(["-t"])
        .write_stdin(archive_bytes)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(file_path));
}

#[test]
fn should_decode_content_of_tar_made_of_one_file() {
    let dir = create_temp_dir();
    let content = "Hello World";
    let file = create_temp_file(&dir, content);
    let file_path = &get_relative_path(&file);
    let archive_name = &format!("{}/archive.tar", { dir.path().to_str().unwrap() });

    ProcessCommand::new("tar")
        .args(["-cf", archive_name, file_path])
        .output()
        .expect("error creating tar archive");

    let mut cmd = Command::cargo_bin("ctar").expect("error getting ctar binary");
    cmd.args(["-tf", archive_name])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(file_path));
}

#[test]
fn should_decode_content_of_tar_made_of_two_file() {
    let dir = create_temp_dir();
    let file_content1 = "Hello World";
    let file1 = create_temp_file(&dir, file_content1);
    let file_path1 = &get_relative_path(&file1);
    let file_content2 = "I have a dream!";
    let file2 = create_temp_file(&dir, file_content2);
    let file_path2 = &get_relative_path(&file2);
    let archive_name = &format!("{}/archive.tar", { dir.path().to_str().unwrap() });

    ProcessCommand::new("tar")
        .args(["-cf", archive_name, file_path1, file_path2])
        .output()
        .expect("error creating tar archive");

    let mut cmd = Command::cargo_bin("ctar").expect("error getting ctar binary");

    cmd.args(["-tf", archive_name])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(file_path1).and(predicate::str::contains(file_path2)));
}
