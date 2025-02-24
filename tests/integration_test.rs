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

#[test]
fn should_decode_content_of_tar_from_stdin() {
    let dir = create_temp_dir();
    let content = "Hello World";
    let file = create_temp_file(&dir, content);
    let file_name = file
        .path()
        .file_name()
        .expect("error extracting file name")
        .to_str()
        .expect("error converting file name to string");
    let archive_name = "archive.tar";
    let dir_path = dir
        .path()
        .to_str()
        .expect("error converting dir path to string");

    ProcessCommand::new("tar")
        .current_dir(dir.path())
        .args(["-cf", archive_name, file_name])
        .output()
        .expect("error creating tar archive");

    let archive_bytes = std::fs::read(format!("{dir_path}/{archive_name}"))
        .expect("error reading tar archive from disk");

    let mut cmd = Command::cargo_bin("ctar").expect("error getting ctar binary");
    cmd.args(["-t"])
        .current_dir(dir.path())
        .write_stdin(archive_bytes)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(file_name));
}

#[test]
fn should_decode_content_of_tar_made_of_one_file() {
    let dir = create_temp_dir();
    let content = "Hello World";
    let file = create_temp_file(&dir, content);
    let file_name = file
        .path()
        .file_name()
        .expect("error extracting file name")
        .to_str()
        .expect("error converting file name to string");
    let archive_name = "archive.tar";

    ProcessCommand::new("tar")
        .current_dir(dir.path())
        .args(["-cf", archive_name, file_name])
        .output()
        .expect("error creating tar archive");

    let mut cmd = Command::cargo_bin("ctar").expect("error getting ctar binary");
    cmd.args(["-tf", archive_name])
        .current_dir(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains(file_name));
}

#[test]
fn should_decode_content_of_tar_made_of_two_file() {
    let dir = create_temp_dir();
    let file_content1 = "Hello World";
    let file1 = create_temp_file(&dir, file_content1);
    let file_name1 = file1
        .path()
        .file_name()
        .expect("error extracting file name")
        .to_str()
        .expect("error converting file name to string");
    let file_content2 = "I have a dream!";
    let file2 = create_temp_file(&dir, file_content2);
    let file_name2 = file2
        .path()
        .file_name()
        .expect("error extracting file name")
        .to_str()
        .expect("error converting file name to string");
    let archive_name = "archive.tar";

    ProcessCommand::new("tar")
        .current_dir(&dir)
        .args(["-cf", archive_name, file_name1, file_name2])
        .output()
        .expect("error creating tar archive");

    let mut cmd = Command::cargo_bin("ctar").expect("error getting ctar binary");

    cmd.args(["-tf", archive_name])
        .current_dir(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains(file_name1).and(predicate::str::contains(file_name2)));
}

#[test]
fn should_extract_file_from_tar() {
    let dir = create_temp_dir();
    let dir_path = dir
        .path()
        .to_str()
        .expect("error converting dir path to string");
    let file_content1 = "Hello World";
    let file = create_temp_file(&dir, file_content1);

    let archive_name = "archive.tar";

    let file_name = file
        .path()
        .file_name()
        .expect("error extracting file name")
        .to_str()
        .expect("error converting file name to string");

    ProcessCommand::new("tar")
        .current_dir(&dir)
        .args(["-cf", archive_name, file_name])
        .output()
        .expect("error creating tar archive");

    let dir2 = create_temp_dir();
    let mut cmd = Command::cargo_bin("ctar").expect("error getting ctar binary");

    cmd.args(["-xf", &format!("{dir_path}/{archive_name}")])
        .current_dir(dir2.path())
        .assert()
        .code(0);

    let extracted_file_path = dir2.path().join(&file_name);

    let extracted_content =
        std::fs::read_to_string(&extracted_file_path).expect("error reading extracted file");

    assert_eq!(extracted_content, file_content1);
}
