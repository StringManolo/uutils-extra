use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn is_executable(path: &Path) -> bool {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(windows)]
    {
        true
    }

    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

#[cfg(windows)]
fn extensions() -> Vec<String> {
    env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(windows)]
fn candidates(name: &OsStr) -> Vec<PathBuf> {
    let path = PathBuf::from(name);

    if path.extension().is_some() {
        return vec![path];
    }

    let mut result = vec![path.clone()];

    for ext in extensions() {
        let mut candidate = path.clone();
        candidate.set_extension(ext.trim_start_matches('.'));
        result.push(candidate);
    }

    result
}

#[cfg(not(windows))]
fn candidates(name: &OsStr) -> Vec<PathBuf> {
    vec![PathBuf::from(name)]
}

fn has_path_separator(name: &OsStr) -> bool {
    let name = name.to_string_lossy();

    #[cfg(windows)]
    {
        name.contains('/') || name.contains('\\')
    }

    #[cfg(not(windows))]
    {
        name.contains('/')
    }
}

fn find_all(name: &OsStr) -> Vec<PathBuf> {
    let mut found = Vec::new();

    // Explicit paths: ./program, /usr/bin/program, etc.
    if has_path_separator(name) {
        for candidate in candidates(name) {
            if is_executable(&candidate) {
                found.push(candidate);
            }
        }

        return found;
    }

    let path_var = match env::var_os("PATH") {
        Some(p) => p,
        None => return found,
    };

    for directory in env::split_paths(&path_var) {
        for candidate in candidates(name) {
            let full_path = directory.join(candidate);

            if is_executable(&full_path) {
                found.push(full_path);
            }
        }
    }

    found
}

fn print_help() {
    println!(
        "which {}\n\
         Find executable files in PATH.\n\
         \n\
         Usage: which [OPTION]... NAME...\n\
         \n\
         Options:\n\
           -a, --all       Print all matching executables\n\
           -s, --silent    Suppress output\n\
           -h, --help      Display this help\n\
           -V, --version   Display version information",
        VERSION
    );
}

pub fn main() {
    let mut all = false;
    let mut silent = false;
    let mut names = Vec::new();
    let mut options = true;

    for arg in env::args_os().skip(1) {
        if options {
            if arg == "--" {
                options = false;
                continue;
            }

            if arg == "-a" || arg == "--all" {
                all = true;
                continue;
            }

            if arg == "-s" || arg == "--silent" {
                silent = true;
                continue;
            }

            if arg == "-h" || arg == "--help" {
                print_help();
                return;
            }

            if arg == "-V" || arg == "--version" {
                println!("which {}", VERSION);
                return;
            }

            if arg == OsStr::new("-") {
                options = false;
                names.push(arg);
                continue;
            }

            if arg.to_string_lossy().starts_with('-') {
                eprintln!("which: unknown option: {}", arg.to_string_lossy());
                std::process::exit(2);
            }
        }

        names.push(arg);
    }

    if names.is_empty() {
        if !silent {
            eprintln!("which: missing program name");
        }
        std::process::exit(2);
    }

    let mut exit_code = 0;

    for name in names {
        let matches = find_all(&name);

        if matches.is_empty() {
            exit_code = 1;
            continue;
        }

        if silent {
            continue;
        }

        if all {
            for path in matches {
                println!("{}", path.display());
            }
        } else {
            println!("{}", matches[0].display());
        }
    }

    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);

            let path =
                env::temp_dir().join(format!("uutils-extra-which-{}-{}", std::process::id(), id));

            fs::create_dir_all(&path).unwrap();

            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn create_file(&self, name: &str) -> PathBuf {
            let path = self.path().join(name);
            fs::write(&path, b"test").unwrap();
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finds_existing_executable() {
        let dir = TestDir::new();
        let path = dir.create_file("program");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        assert!(is_executable(&path));
    }

    #[test]
    fn rejects_missing_file() {
        let dir = TestDir::new();
        let path = dir.path().join("does-not-exist");

        assert!(!is_executable(&path));
    }

    #[test]
    fn rejects_directories() {
        let dir = TestDir::new();

        assert!(!is_executable(dir.path()));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_file_without_execute_permission() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestDir::new();
        let path = dir.create_file("not-executable");

        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(!is_executable(&path));
    }

    #[test]
    fn finds_explicit_executable_path() {
        let dir = TestDir::new();
        let path = dir.create_file("program");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let results = find_all(path.as_os_str());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], path);
    }

    #[test]
    fn does_not_find_non_executable_explicit_path() {
        let dir = TestDir::new();
        let path = dir.create_file("program");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        }

        let results = find_all(path.as_os_str());

        assert!(results.is_empty());
    }

    #[test]
    fn candidates_returns_at_least_one_path() {
        let result = candidates(OsStr::new("program"));

        assert!(!result.is_empty());
    }
}
