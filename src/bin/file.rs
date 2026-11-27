use std::env;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(unix)]
fn describe(path: &Path) -> String {
    use std::os::unix::fs::FileTypeExt;

    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => return format!("cannot open: {}", e),
    };

    let kind = metadata.file_type();

    if kind.is_symlink() {
        return match fs::read_link(path) {
            Ok(target) => format!("symbolic link to {}", target.display()),
            Err(_) => "symbolic link".into(),
        };
    }

    if kind.is_dir() {
        return "directory".into();
    }

    if kind.is_fifo() {
        return "FIFO".into();
    }

    if kind.is_socket() {
        return "socket".into();
    }

    if kind.is_block_device() {
        return "block special".into();
    }

    if kind.is_char_device() {
        return "character special".into();
    }

    if !kind.is_file() {
        return "special file".into();
    }

    describe_file_contents(path)
}

#[cfg(not(unix))]
fn describe(path: &Path) -> String {
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => return format!("cannot open: {}", e),
    };

    if metadata.file_type().is_symlink() {
        return "symbolic link".into();
    }

    if metadata.is_dir() {
        return "directory".into();
    }

    if metadata.is_file() {
        return describe_file_contents(path);
    }

    "special file".into()
}

fn describe_file_contents(path: &Path) -> String {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(e) => return format!("cannot read: {}", e),
    };

    if data.is_empty() {
        return "empty".into();
    }

    if data.starts_with(b"\x7fELF") {
        if data.len() > 5 {
            let bits = match data[4] {
                1 => "32-bit",
                2 => "64-bit",
                _ => "unknown-bit",
            };

            let endian = match data[5] {
                1 => "LSB",
                2 => "MSB",
                _ => "unknown-endian",
            };

            return format!("ELF {}, {}", bits, endian);
        }

        return "ELF executable".into();
    }

    if data.starts_with(b"MZ") {
        return "DOS/Windows executable (PE)".into();
    }

    if data.starts_with(b"%PDF-") {
        return "PDF document".into();
    }

    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return "PNG image".into();
    }

    if data.starts_with(b"\xff\xd8\xff") {
        return "JPEG image".into();
    }

    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return "GIF image".into();
    }

    if data.starts_with(b"PK\x03\x04") {
        return "ZIP archive".into();
    }

    if data.starts_with(b"\x1f\x8b") {
        return "gzip compressed data".into();
    }

    if data.starts_with(b"7z\xBC\xAF\x27\x1C") {
        return "7-zip archive".into();
    }

    if data.starts_with(b"Rar!\x1A\x07") {
        return "RAR archive".into();
    }

    if data.starts_with(b"SQLite format 3\0") {
        return "SQLite database".into();
    }

    if data.starts_with(b"#!") {
        let first_line = String::from_utf8_lossy(&data)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        return format!("script, {}", first_line);
    }

    if std::str::from_utf8(&data).is_ok() {
        if data.iter().all(|b| {
            b.is_ascii_graphic()
                || b.is_ascii_whitespace()
                || *b >= 0x80
        }) {
            return "Unicode/UTF-8 text".into();
        }

        return "UTF-8 text with control characters".into();
    }

    "data".into()
}

pub fn main() {
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!(
                    "file {}\nUsage: file FILE...",
                    VERSION
                );
                return;
            }
            "-v" | "--version" => {
                println!("file {}", VERSION);
                return;
            }
            _ if arg.starts_with('-') && arg != "-" => {
                eprintln!("file: unsupported option: {}", arg);
                std::process::exit(2);
            }
            _ => {
                let path = Path::new(&arg);
                println!("{}: {}", arg, describe(path));
            }
        }
    }
}
