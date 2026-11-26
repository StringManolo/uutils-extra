use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct Options {
    name: Option<String>,
    case_insensitive: bool,
    file_type: Option<char>,
    max_depth: Option<usize>,
    min_depth: usize,
    empty: bool,
    print0: bool,
    negate: bool,
}

fn print_help() {
    println!(
        "find {}\n\
         Search for files in a directory hierarchy.\n\
         \n\
         Usage: find [PATH...] [EXPRESSION]\n\
         \n\
         Expressions:\n\
           -name PATTERN       Match file name\n\
           -iname PATTERN      Match file name, ignoring case\n\
           -type TYPE          Match type: f, d or l\n\
           -maxdepth N         Maximum directory depth\n\
           -mindepth N         Minimum directory depth\n\
           -empty              Match empty files or directories\n\
           -not, !             Negate the next test\n\
           -print              Print matching paths (default)\n\
           -print0             Print paths separated by NUL\n\
           -help               Display this help\n\
           -version            Display version information",
        VERSION
    );
}

fn wildcard_match(pattern: &str, text: &str, insensitive: bool) -> bool {
    let pattern = if insensitive {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };

    let text = if insensitive {
        text.to_lowercase()
    } else {
        text.to_string()
    };

    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();

    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;

    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }

        for j in 1..=t.len() {
            dp[i][j] = match p[i - 1] {
                '*' => dp[i - 1][j] || dp[i][j - 1],
                '?' => dp[i - 1][j - 1],
                c => c == t[j - 1] && dp[i - 1][j - 1],
            };
        }
    }

    dp[p.len()][t.len()]
}

fn matches(path: &Path, depth: usize, options: &Options) -> bool {
    if depth < options.min_depth {
        return false;
    }

    if let Some(max) = options.max_depth {
        if depth > max {
            return false;
        }
    }

    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    let mut result = true;

    if let Some(pattern) = &options.name {
        let name = path.file_name().unwrap_or(OsStr::new("")).to_string_lossy();

        result &= wildcard_match(pattern, &name, options.case_insensitive);
    }

    if let Some(kind) = options.file_type {
        let kind_matches = match kind {
            'f' => metadata.file_type().is_file(),
            'd' => metadata.file_type().is_dir(),
            'l' => metadata.file_type().is_symlink(),
            _ => false,
        };

        result &= kind_matches;
    }

    if options.empty {
        let is_empty = if metadata.is_file() {
            metadata.len() == 0
        } else if metadata.is_dir() {
            fs::read_dir(path)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false)
        } else {
            false
        };

        result &= is_empty;
    }

    if options.negate {
        !result
    } else {
        result
    }
}

fn walk(path: &Path, depth: usize, options: &Options, output: &mut Vec<PathBuf>) {
    if matches(path, depth, options) {
        output.push(path.to_path_buf());
    }

    if let Some(max) = options.max_depth {
        if depth >= max {
            return;
        }
    }

    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return,
    };

    // Do not follow symbolic links to directories.
    if !metadata.file_type().is_dir() {
        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        walk(&entry.path(), depth + 1, options, output);
    }
}

fn parse_args() -> Result<(Vec<PathBuf>, Options), String> {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut paths = Vec::new();
    let mut options = Options::default();
    let mut i = 0;
    let mut expression_started = false;

    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "-help" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-version" | "--version" => {
                println!("find {}", VERSION);
                std::process::exit(0);
            }
            "-name" | "-iname" => {
                expression_started = true;
                i += 1;

                if i >= args.len() {
                    return Err(format!("{} requires a pattern", arg));
                }

                options.name = Some(args[i].clone());
                options.case_insensitive = arg == "-iname";
            }
            "-type" => {
                expression_started = true;
                i += 1;

                if i >= args.len() {
                    return Err("-type requires an argument".to_string());
                }

                let kind = args[i].chars().next().unwrap_or('\0');

                if !matches!(kind, 'f' | 'd' | 'l') {
                    return Err(format!("invalid type: {}", args[i]));
                }

                options.file_type = Some(kind);
            }
            "-maxdepth" | "-mindepth" => {
                expression_started = true;
                i += 1;

                if i >= args.len() {
                    return Err(format!("{} requires a number", arg));
                }

                let value = args[i]
                    .parse::<usize>()
                    .map_err(|_| format!("invalid depth: {}", args[i]))?;

                if arg == "-maxdepth" {
                    options.max_depth = Some(value);
                } else {
                    options.min_depth = value;
                }
            }
            "-empty" => {
                expression_started = true;
                options.empty = true;
            }
            "-not" | "!" => {
                expression_started = true;
                options.negate = !options.negate;
            }
            "-print" => {
                expression_started = true;
            }
            "-print0" => {
                expression_started = true;
                options.print0 = true;
            }
            _ => {
                if expression_started {
                    return Err(format!("unknown expression: {}", arg));
                }

                paths.push(PathBuf::from(arg));
            }
        }

        i += 1;
    }

    if paths.is_empty() {
        paths.push(PathBuf::from("."));
    }

    Ok((paths, options))
}

fn main() {
    let (paths, options) = match parse_args() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("find: {}", error);
            std::process::exit(2);
        }
    };

    let mut results = Vec::new();

    for path in paths {
        walk(&path, 0, &options, &mut results);
    }

    for path in results {
        if options.print0 {
            use std::io::Write;
            let _ = std::io::stdout().write_all(path.as_os_str().to_string_lossy().as_bytes());
            let _ = std::io::stdout().write_all(b"\0");
        } else {
            println!("{}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "uutils-extra-find-{}-{}",
                std::process::id(),
                id
            ));

            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
            let path = self.path().join(name);
            fs::write(&path, content).unwrap();
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn matches_wildcards() {
        assert!(wildcard_match("*.rs", "main.rs", false));
        assert!(wildcard_match("file?.txt", "file1.txt", false));
        assert!(!wildcard_match("*.rs", "main.txt", false));
    }

    #[test]
    fn matches_case_insensitively() {
        assert!(wildcard_match("*.RS", "main.rs", true));
    }

    #[test]
    fn finds_files_by_name() {
        let dir = TestDir::new();
        let file = dir.create_file("hello.txt", b"hello");

        let options = Options {
            name: Some("*.txt".to_string()),
            ..Options::default()
        };

        let mut results = Vec::new();
        walk(dir.path(), 0, &options, &mut results);

        assert!(results.contains(&file));
    }

    #[test]
    fn finds_empty_files() {
        let dir = TestDir::new();
        let file = dir.create_file("empty.txt", b"");

        let options = Options {
            empty: true,
            ..Options::default()
        };

        let mut results = Vec::new();
        walk(dir.path(), 0, &options, &mut results);

        assert!(results.contains(&file));
    }

    #[test]
    fn respects_max_depth() {
        let dir = TestDir::new();
        let nested = dir.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        let file = nested.join("file.txt");
        fs::write(&file, b"data").unwrap();

        let options = Options {
            max_depth: Some(0),
            ..Options::default()
        };

        let mut results = Vec::new();
        walk(dir.path(), 0, &options, &mut results);

        assert_eq!(results, vec![dir.path().to_path_buf()]);
    }

    #[test]
    fn matches_file_type() {
        let dir = TestDir::new();
        let file = dir.create_file("file.txt", b"data");

        let options = Options {
            file_type: Some('f'),
            ..Options::default()
        };

        assert!(matches(&file, 1, &options));
        assert!(!matches(dir.path(), 0, &options));
    }
}

