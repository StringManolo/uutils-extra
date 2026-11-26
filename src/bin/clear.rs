use std::io::{self, Write};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("clear {}\nUsage: clear", VERSION);
                return;
            }
            "-V" | "--version" => {
                println!("clear {}", VERSION);
                return;
            }
            _ => {
                eprintln!("clear: unsupported option: {}", arg);
                std::process::exit(2);
            }
        }
    }

    print!("\x1bc");

    if let Err(e) = io::stdout().flush() {
        eprintln!("clear: {}", e);
        std::process::exit(1);
    }
}
