use regex::Regex;
use std::env;
use std::fs;
use std::io::{self, BufRead};

const VERSION: &str = env!("CARGO_PKG_VERSION");

enum Command {
    Substitute {
        regex: Regex,
        replacement: String,
        global: bool,
    },
    Delete,
    Print,
}

fn parse_substitution(expr: &str) -> Result<Command, String> {
    let mut chars = expr.chars();

    if chars.next() != Some('s') {
        return Err("expected substitution command".into());
    }

    let delimiter = chars.next().ok_or("missing delimiter")?;

    let parts: Vec<&str> = expr[2..].split(delimiter).collect();

    if parts.len() < 3 {
        return Err("invalid substitution expression".into());
    }

    let regex = Regex::new(parts[0])
        .map_err(|e| format!("invalid regex: {}", e))?;

    let replacement = parts[1].replace("\\1", "$1");
    let global = parts[2].contains('g');

    Ok(Command::Substitute {
        regex,
        replacement,
        global,
    })
}

fn parse_command(expr: &str) -> Result<Command, String> {
    let expr = expr.trim();

    if expr == "d" {
        return Ok(Command::Delete);
    }

    if expr == "p" {
        return Ok(Command::Print);
    }

    if expr.starts_with("s") {
        return parse_substitution(expr);
    }

    Err(format!("unsupported expression: {}", expr))
}

fn process<R: BufRead>(
    reader: R,
    commands: &[Command],
    quiet: bool,
) {
    for line in reader.lines() {
        let mut line = match line {
            Ok(line) => line,
            Err(_) => continue,
        };

        let mut deleted = false;
        let mut explicitly_printed = false;

        for command in commands {
            match command {
                Command::Substitute {
                    regex,
                    replacement,
                    global,
                } => {
                    if *global {
                        line = regex
                            .replace_all(&line, replacement.as_str())
                            .into_owned();
                    } else {
                        line = regex
                            .replace(&line, replacement.as_str())
                            .into_owned();
                    }
                }
                Command::Delete => {
                    deleted = true;
                    break;
                }
                Command::Print => {
                    println!("{}", line);
                    explicitly_printed = true;
                }
            }
        }

        if !deleted && !quiet && !explicitly_printed {
            println!("{}", line);
        }
    }
}

fn help() {
    println!(
        "sed {}\n\
         Usage: sed [OPTION]... SCRIPT [FILE...]\n\
         Options:\n\
           -n          Suppress automatic printing\n\
           -e SCRIPT   Add a script expression\n\
           -h, --help  Display help\n\
           -V, --version Display version",
        VERSION
    );
}

fn main() {
    let mut quiet = false;
    let mut expressions = Vec::new();
    let mut files = Vec::new();
    let mut script_found = false;

    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                help();
                return;
            }
            "-V" | "--version" => {
                println!("sed {}", VERSION);
                return;
            }
            "-n" => quiet = true,
            "-e" => {
                let expr = match args.next() {
                    Some(e) => e,
                    None => {
                        eprintln!("sed: -e requires an expression");
                        std::process::exit(2);
                    }
                };
                expressions.push(expr);
                script_found = true;
            }
            _ if arg.starts_with('-') && arg != "-" => {
                eprintln!("sed: unknown option: {}", arg);
                std::process::exit(2);
            }
            _ if !script_found => {
                expressions.push(arg);
                script_found = true;
            }
            _ => files.push(arg),
        }
    }

    if expressions.is_empty() {
        eprintln!("sed: missing script");
        std::process::exit(2);
    }

    let commands: Vec<Command> = match expressions
        .iter()
        .map(|e| parse_command(e))
        .collect()
    {
        Ok(commands) => commands,
        Err(e) => {
            eprintln!("sed: {}", e);
            std::process::exit(2);
        }
    };

    if files.is_empty() {
        process(io::stdin().lock(), &commands, quiet);
        return;
    }

    for filename in files {
        if filename == "-" {
            process(io::stdin().lock(), &commands, quiet);
            continue;
        }

        match fs::File::open(&filename) {
            Ok(file) => {
                process(io::BufReader::new(file), &commands, quiet);
            }
            Err(e) => {
                eprintln!("sed: {}: {}", filename, e);
                std::process::exit(2);
            }
        }
    }
}
