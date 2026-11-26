use regex::{Regex, RegexBuilder};
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct Options {
    ignore_case: bool,
    invert: bool,
    line_number: bool,
    count: bool,
    files_with_matches: bool,
    files_without_matches: bool,
    only_matching: bool,
    quiet: bool,
}

fn help() {
    println!(
        "grep {}\n\
         Usage: grep [OPTION]... PATTERN [FILE...]\n\
         Options:\n\
           -i, --ignore-case       Ignore case distinctions\n\
           -v, --invert-match      Select non-matching lines\n\
           -n, --line-number       Print line numbers\n\
           -c, --count             Print match count\n\
           -l, --files-with-matches Print matching file names\n\
           -L, --files-without-match Print non-matching file names\n\
           -o, --only-matching     Print only matched parts\n\
           -q, --quiet             Suppress output\n\
           -h, --help              Display help\n\
           -V, --version           Display version",
        VERSION
    );
}

fn process<R: BufRead>(
    reader: R,
    label: &str,
    regex: &Regex,
    options: &Options,
    show_filename: bool,
) -> bool {
    let mut count = 0usize;

    for (index, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(_) => continue,
        };

        let matched = regex.is_match(&line);
        let selected = if options.invert { !matched } else { matched };

        if !selected {
            continue;
        }

        count += 1;

        if options.quiet {
            return true;
        }

        if options.files_with_matches || options.files_without_matches {
            continue;
        }

        if options.only_matching && !options.invert {
            for found in regex.find_iter(&line) {
                let mut prefix = String::new();

                if show_filename {
                    prefix.push_str(label);
                    prefix.push(':');
                }

                if options.line_number {
                    prefix.push_str(&format!("{}:", index + 1));
                }

                println!("{}{}", prefix, found.as_str());
            }

            continue;
        }

        let mut prefix = String::new();

        if show_filename {
            prefix.push_str(label);
            prefix.push(':');
        }

        if options.line_number {
            prefix.push_str(&format!("{}:", index + 1));
        }

        println!("{}{}", prefix, line);
    }

    if options.count && !options.quiet {
        if show_filename {
            println!("{}:{}", label, count);
        } else {
            println!("{}", count);
        }
    }

    if options.files_with_matches && count > 0 {
        println!("{}", label);
    }

    if options.files_without_matches && count == 0 {
        println!("{}", label);
    }

    count > 0
}

fn main() {
    let mut options = Options::default();
    let mut pattern = None;
    let mut files = Vec::new();
    let mut parsing_options = true;

    for arg in env::args().skip(1) {
        if parsing_options && arg == "--" {
            parsing_options = false;
            continue;
        }

        if parsing_options {
            match arg.as_str() {
                "-h" | "--help" => {
                    help();
                    return;
                }
                "-V" | "--version" => {
                    println!("grep {}", VERSION);
                    return;
                }
                "-i" | "--ignore-case" => options.ignore_case = true,
                "-v" | "--invert-match" => options.invert = true,
                "-n" | "--line-number" => options.line_number = true,
                "-c" | "--count" => options.count = true,
                "-l" | "--files-with-matches" => {
                    options.files_with_matches = true
                }
                "-L" | "--files-without-match" => {
                    options.files_without_matches = true
                }
                "-o" | "--only-matching" => options.only_matching = true,
                "-q" | "--quiet" => options.quiet = true,
                _ if arg.starts_with('-') && arg != "-" => {
                    eprintln!("grep: unknown option: {}", arg);
                    std::process::exit(2);
                }
                _ => {
                    if pattern.is_none() {
                        pattern = Some(arg);
                    } else {
                        files.push(arg);
                    }
                }
            }
        } else if pattern.is_none() {
            pattern = Some(arg);
        } else {
            files.push(arg);
        }
    }

    let pattern = match pattern {
        Some(p) => p,
        None => {
            eprintln!("grep: missing pattern");
            std::process::exit(2);
        }
    };

    let regex = match RegexBuilder::new(&pattern)
        .case_insensitive(options.ignore_case)
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("grep: invalid regex: {}", e);
            std::process::exit(2);
        }
    };

    let mut found = false;
    let show_filename = files.len() > 1;

    if files.is_empty() || files.iter().any(|f| f == "-") {
        let stdin = io::stdin();
        found |= process(
            stdin.lock(),
            "(standard input)",
            &regex,
            &options,
            false,
        );
    }

    for filename in files.iter().filter(|f| *f != "-") {
        let path = Path::new(filename);

        match fs::File::open(path) {
            Ok(file) => {
                found |= process(
                    io::BufReader::new(file),
                    filename,
                    &regex,
                    &options,
                    show_filename,
                );
            }
            Err(e) => {
                eprintln!("grep: {}: {}", filename, e);
            }
        }
    }

    if options.quiet && found {
        std::process::exit(0);
    }

    std::process::exit(if found { 0 } else { 1 });
}
