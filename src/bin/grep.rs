use regex::{Regex, RegexBuilder};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, PartialEq, Default)]
enum ColorMode {
    Never,
    Always,
    #[default]
    Auto,
}

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
    word_regexp: bool,
    line_regexp: bool,
    recursive: bool,
    fixed_strings: bool,
    text: bool,
    no_messages: bool,
    with_filename: Option<bool>,
    max_count: Option<usize>,
    color: ColorMode,
    include: Vec<String>,
    exclude: Vec<String>,
}

struct ParsedArgs {
    options: Options,
    patterns: Vec<String>,
    files: Vec<String>,
}

fn help() {
    println!(
        "grep {}\n\
         Usage: grep [OPTION]... PATTERN [FILE...]\n\
         Options:\n\
           -E, --extended-regexp     PATTERN is an extended regular expression\n\
           -F, --fixed-strings       PATTERN is a set of newline-separated strings\n\
           -i, --ignore-case         Ignore case distinctions\n\
           -v, --invert-match        Select non-matching lines\n\
           -w, --word-regexp         Match only whole words\n\
           -x, --line-regexp         Match only whole lines\n\
           -n, --line-number         Print line numbers\n\
           -c, --count               Print match count per file\n\
           -l, --files-with-matches  Print only names of files with matches\n\
           -L, --files-without-match Print only names of files without matches\n\
           -o, --only-matching       Print only matched parts of a line\n\
           -q, --quiet, --silent     Suppress all normal output\n\
           -r, -R, --recursive       Read all files under each directory\n\
           -a, --text                Treat binary files as text\n\
           -s, --no-messages         Suppress error messages\n\
           -H, --with-filename       Print the file name for each match\n\
           -h, --no-filename         Suppress the file name prefix\n\
           -e, --regexp=PATTERN      Use PATTERN as the pattern\n\
           -m, --max-count=NUM       Stop after NUM matches per file\n\
               --include=GLOB        Only search files matching GLOB\n\
               --exclude=GLOB        Skip files matching GLOB\n\
               --color[=WHEN]        Highlight matches (WHEN: always/never/auto)\n\
           -V, --version             Display version\n\
               --help                Display this help",
        VERSION
    );
}

fn parse_args() -> ParsedArgs {
    let mut o = Options::default();
    let mut patterns: Vec<String> = Vec::new();
    let mut files: Vec<String> = Vec::new();
    let mut used_e = false;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    let mut no_more_opts = false;

    while i < args.len() {
        let arg = &args[i];

        if no_more_opts {
            if !used_e && patterns.is_empty() {
                patterns.push(arg.clone());
            } else {
                files.push(arg.clone());
            }
            i += 1;
            continue;
        }

        if arg == "--" {
            no_more_opts = true;
            i += 1;
            continue;
        }

        if arg == "-" || !arg.starts_with('-') {
            if !used_e && patterns.is_empty() {
                patterns.push(arg.clone());
            } else {
                files.push(arg.clone());
            }
            i += 1;
            continue;
        }

        if let Some(long) = arg.strip_prefix("--") {
            let (name, val) = match long.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (long, None),
            };
            let take_val = |i: &mut usize| -> Option<String> {
                if let Some(v) = &val {
                    return Some(v.clone());
                }
                *i += 1;
                args.get(*i).cloned()
            };
            match name {
                "help" => { help(); std::process::exit(0); }
                "version" => { println!("grep {}", VERSION); std::process::exit(0); }
                "ignore-case" => o.ignore_case = true,
                "invert-match" => o.invert = true,
                "line-number" => o.line_number = true,
                "count" => o.count = true,
                "files-with-matches" => o.files_with_matches = true,
                "files-without-match" => o.files_without_matches = true,
                "only-matching" => o.only_matching = true,
                "quiet" | "silent" => o.quiet = true,
                "word-regexp" => o.word_regexp = true,
                "line-regexp" => o.line_regexp = true,
                "extended-regexp" | "basic-regexp" | "perl-regexp" => {}
                "fixed-strings" => o.fixed_strings = true,
                "recursive" | "dereference-recursive" => o.recursive = true,
                "text" => o.text = true,
                "no-messages" => o.no_messages = true,
                "with-filename" => o.with_filename = Some(true),
                "no-filename" => o.with_filename = Some(false),
                "color" | "colour" => {
                    o.color = match val.as_deref() {
                        None | Some("auto") | Some("tty") | Some("if-tty") => ColorMode::Auto,
                        Some("always") | Some("yes") | Some("force") => ColorMode::Always,
                        Some("never") | Some("no") | Some("none") => ColorMode::Never,
                        Some(other) => {
                            eprintln!("grep: invalid color mode: {}", other);
                            std::process::exit(2);
                        }
                    };
                }
                "regexp" => {
                    match take_val(&mut i) {
                        Some(v) => { patterns.push(v); used_e = true; }
                        None => { eprintln!("grep: option '--regexp' requires an argument"); std::process::exit(2); }
                    }
                }
                "max-count" => {
                    match take_val(&mut i).and_then(|v| v.parse::<usize>().ok()) {
                        Some(v) => o.max_count = Some(v),
                        None => { eprintln!("grep: invalid max count"); std::process::exit(2); }
                    }
                }
                "include" => {
                    match take_val(&mut i) {
                        Some(v) => o.include.push(v),
                        None => { eprintln!("grep: option '--include' requires an argument"); std::process::exit(2); }
                    }
                }
                "exclude" => {
                    match take_val(&mut i) {
                        Some(v) => o.exclude.push(v),
                        None => { eprintln!("grep: option '--exclude' requires an argument"); std::process::exit(2); }
                    }
                }
                _ => {
                    eprintln!("grep: unrecognized option '--{}'", name);
                    std::process::exit(2);
                }
            }
            i += 1;
            continue;
        }

        // Short options: -XYZ combined, or -Xvalue, or -X value
        let chars: Vec<char> = arg.chars().skip(1).collect();
        let mut j = 0;
        while j < chars.len() {
            let c = chars[j];
            match c {
                'i' => o.ignore_case = true,
                'v' => o.invert = true,
                'n' => o.line_number = true,
                'c' => o.count = true,
                'l' => o.files_with_matches = true,
                'L' => o.files_without_matches = true,
                'o' => o.only_matching = true,
                'q' => o.quiet = true,
                'w' => o.word_regexp = true,
                'x' => o.line_regexp = true,
                'E' | 'G' | 'P' => {}
                'F' => o.fixed_strings = true,
                'r' | 'R' => o.recursive = true,
                'a' => o.text = true,
                's' => o.no_messages = true,
                'H' => o.with_filename = Some(true),
                'h' => o.with_filename = Some(false),
                'V' => { println!("grep {}", VERSION); std::process::exit(0); }
                'e' => {
                    let rest: String = chars[j + 1..].iter().collect();
                    if !rest.is_empty() {
                        patterns.push(rest);
                    } else {
                        i += 1;
                        match args.get(i) {
                            Some(v) => patterns.push(v.clone()),
                            None => { eprintln!("grep: option '-e' requires an argument"); std::process::exit(2); }
                        }
                    }
                    used_e = true;
                    break;
                }
                'm' => {
                    let rest: String = chars[j + 1..].iter().collect();
                    let val = if !rest.is_empty() {
                        rest
                    } else {
                        i += 1;
                        match args.get(i) {
                            Some(v) => v.clone(),
                            None => { eprintln!("grep: option '-m' requires an argument"); std::process::exit(2); }
                        }
                    };
                    match val.parse::<usize>() {
                        Ok(v) => o.max_count = Some(v),
                        Err(_) => { eprintln!("grep: invalid max count: {}", val); std::process::exit(2); }
                    }
                    break;
                }
                _ => {
                    eprintln!("grep: unknown option: -{}", c);
                    std::process::exit(2);
                }
            }
            j += 1;
        }
        i += 1;
    }

    if patterns.is_empty() {
        eprintln!("grep: missing pattern");
        std::process::exit(2);
    }

    ParsedArgs { options: o, patterns, files }
}

fn build_regex(patterns: &[String], o: &Options) -> Regex {
    let mut alts: Vec<String> = Vec::with_capacity(patterns.len());
    for p in patterns {
        let mut p = if o.fixed_strings {
            regex::escape(p)
        } else {
            p.clone()
        };
        if o.word_regexp {
            p = format!(r"\b(?:{})\b", p);
        }
        if o.line_regexp {
            p = format!(r"^(?:{})$", p);
        }
        alts.push(p);
    }
    let pattern = if alts.len() == 1 {
        alts.into_iter().next().unwrap()
    } else {
        format!("(?:{})", alts.join("|"))
    };
    let mut b = RegexBuilder::new(&pattern);
    b.case_insensitive(o.ignore_case);
    match b.build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("grep: invalid regex: {}", e);
            std::process::exit(2);
        }
    }
}

fn use_color(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Never => false,
        ColorMode::Always => true,
        ColorMode::Auto => unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 },
    }
}

fn write_prefix<W: Write>(
    out: &mut W,
    label: &str,
    line_idx: usize,
    options: &Options,
    show_filename: bool,
    color: bool,
) -> io::Result<()> {
    if show_filename {
        if color {
            write!(out, "\x1b[35m{}\x1b[0m\x1b[36m:\x1b[0m", label)?;
        } else {
            write!(out, "{}:", label)?;
        }
    }
    if options.line_number {
        if color {
            write!(out, "\x1b[32m{}\x1b[0m\x1b[36m:\x1b[0m", line_idx + 1)?;
        } else {
            write!(out, "{}:", line_idx + 1)?;
        }
    }
    Ok(())
}

fn write_highlighted<W: Write>(out: &mut W, line: &str, regex: &Regex) -> io::Result<()> {
    let mut last = 0;
    for m in regex.find_iter(line) {
        write!(out, "{}\x1b[1;31m{}\x1b[0m", &line[last..m.start()], m.as_str())?;
        last = m.end();
    }
    writeln!(out, "{}", &line[last..])
}

fn process<R: BufRead, W: Write>(
    reader: R,
    label: &str,
    regex: &Regex,
    options: &Options,
    show_filename: bool,
    color: bool,
    out: &mut W,
) -> io::Result<bool> {
    let mut count = 0usize;
    let suppress_normal = options.files_with_matches
        || options.files_without_matches
        || options.count;

    for (index, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let matched = regex.is_match(&line);
        let selected = if options.invert { !matched } else { matched };
        if !selected {
            continue;
        }
        count += 1;

        if options.quiet {
            return Ok(true);
        }
        if suppress_normal {
            if let Some(m) = options.max_count {
                if count >= m { break; }
            }
            continue;
        }

        if options.only_matching && !options.invert {
            for found in regex.find_iter(&line) {
                write_prefix(out, label, index, options, show_filename, color)?;
                if color {
                    writeln!(out, "\x1b[1;31m{}\x1b[0m", found.as_str())?;
                } else {
                    writeln!(out, "{}", found.as_str())?;
                }
            }
        } else {
            write_prefix(out, label, index, options, show_filename, color)?;
            if color && !options.invert {
                write_highlighted(out, &line, regex)?;
            } else {
                writeln!(out, "{}", line)?;
            }
        }

        if let Some(m) = options.max_count {
            if count >= m { break; }
        }
    }

    let selected = if options.files_without_matches {
        count == 0
    } else {
        count > 0
    };

    if !options.quiet {
        if options.files_with_matches && count > 0 {
            writeln!(out, "{}", label)?;
        } else if options.files_without_matches && count == 0 {
            writeln!(out, "{}", label)?;
        } else if options.count {
            if show_filename {
                writeln!(out, "{}:{}", label, count)?;
            } else {
                writeln!(out, "{}", count)?;
            }
        }
    }

    Ok(selected)
}

fn glob_match(pattern: &str, s: &str) -> bool {
    glob_bytes(pattern.as_bytes(), s.as_bytes())
}

fn glob_bytes(p: &[u8], s: &[u8]) -> bool {
    if p.is_empty() { return s.is_empty(); }
    match p[0] {
        b'*' => {
            for i in 0..=s.len() {
                if glob_bytes(&p[1..], &s[i..]) { return true; }
            }
            false
        }
        b'?' => !s.is_empty() && glob_bytes(&p[1..], &s[1..]),
        c => !s.is_empty() && s[0] == c && glob_bytes(&p[1..], &s[1..]),
    }
}

fn walk_dir<W: Write>(
    dir: &Path,
    regex: &Regex,
    options: &Options,
    show_filename: bool,
    color: bool,
    out: &mut W,
    found: &mut bool,
) -> io::Result<()> {
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = match fs::read_dir(&d) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let mut children: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            children.push(entry.path());
        }
        children.sort();
        for path in children.into_iter().rev() {
            let meta = match fs::metadata(&path) { Ok(m) => m, Err(_) => continue };
            if meta.is_dir() {
                stack.push(path);
                continue;
            }
            if !meta.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !options.include.is_empty()
                && !options.include.iter().any(|p| glob_match(p, name))
            {
                continue;
            }
            if options.exclude.iter().any(|p| glob_match(p, name)) {
                continue;
            }
            let label = path.to_string_lossy().to_string();
            match fs::File::open(&path) {
                Ok(file) => {
                    if process(io::BufReader::new(file), &label, regex, options, show_filename, color, out)? {
                        *found = true;
                    }
                }
                Err(e) => {
                    if !options.no_messages {
                        eprintln!("grep: {}: {}", label, e);
                    }
                }
            }
        }
    }
    Ok(())
}

fn main_impl() -> io::Result<i32> {
    let ParsedArgs { options, patterns, files } = parse_args();
    let regex = build_regex(&patterns, &options);
    let color = use_color(options.color);

    let use_stdin = (files.is_empty() && !options.recursive)
        || files.iter().any(|f| f == "-");
    let mut real_files: Vec<String> = files.iter().filter(|f| *f != "-").cloned().collect();
    if real_files.is_empty() && options.recursive {
        real_files.push(".".to_string());
    }

    let show_filename = match options.with_filename {
        Some(v) => v,
        None => real_files.len() > 1 || options.recursive,
    };
    let stdin_show = match options.with_filename {
        Some(v) => v,
        None => false,
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut found = false;

    if use_stdin {
        let stdin = io::stdin();
        if process(stdin.lock(), "(standard input)", &regex, &options, stdin_show, color, &mut out)? {
            found = true;
        }
    }

    for filename in &real_files {
        let path = Path::new(filename);
        if path.is_dir() {
            if options.recursive {
                walk_dir(path, &regex, &options, show_filename, color, &mut out, &mut found)?;
            } else if !options.no_messages {
                eprintln!("grep: {}: Is a directory", filename);
            }
            continue;
        }
        match fs::File::open(path) {
            Ok(file) => {
                if process(io::BufReader::new(file), filename, &regex, &options, show_filename, color, &mut out)? {
                    found = true;
                }
            }
            Err(e) => {
                if !options.no_messages {
                    eprintln!("grep: {}: {}", filename, e);
                }
            }
        }
    }

    out.flush()?;
    Ok(if found { 0 } else { 1 })
}

pub fn main() {
    match main_impl() {
        Ok(code) => std::process::exit(code),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => std::process::exit(0),
        Err(e) => {
            eprintln!("grep: {}", e);
            std::process::exit(2);
        }
    }
}
