# ctar

ctar is a lightweight Unix tar clone written in Rust. It offers a minimal implementation to create, list, and extract tar archives using the standard tar format.

## Features

- **Create Tarballs:** Package one or more files or directories into a `.tar` archive.
- **List Archive Contents:** Quickly view the contents of an archive without extraction.
- **Extract Files:** Retrieve and restore archived files to disk.
- **Checksum Verification:** Implements checksum validation for tar headers to ensure data integrity.
- **Unix Permissions and Metadata:** Preserves file modes, user IDs, and group IDs using Unix-specific system calls.

## Installation

To build and run `ctar`, you need to have [Rust](https://www.rust-lang.org/) installed on your system.

1. Clone the repository:

   ```bash
   git clone git@github.com:Paul-Dejean/tar.git
   cd tar
   ```

2. Build the project using Cargo:

   ```bash
   cargo build --release
   ```

3. (Optional) Install the binary globally:

   ```bash
   cargo install --path .
   ```

## Usage

Once built, you can run tar from the command line. Below are some example usages:

**Creating an Archive:**

```bash
./target/release/ctar -c -f archive.tar file1.txt file2.txt
```

Or if installed globally:

```bash
cxxd file.txt > file.hex
```

**Listing Archive Contents:**

```bash
./target/release/ctar -t -f archive.tar
```

**Extracting Files:**

```bash
./target/release/ctar -x -f archive.tar
```

## Author

Paul Dejean (pauldejeandev@gmail.com)
