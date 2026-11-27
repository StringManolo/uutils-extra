use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(windows)]
fn extensions() -> Vec<OsString> {
    env::var_os("PATHEXT")
        .unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"))
        .to_string_lossy()
        .split(';')
        .filter(|s| !s.is_empty())
        .map(OsString::from)
        .collect()
}

#[cfg(windows)]
fn candidates(path: &Path) -> Vec<PathBuf> {
    if path.extension().is_some() {
        return vec![path.to_path_buf()];
    }

    let mut result = vec![path.to_path_buf()];

    for ext in extensions() {
        let mut candidate = path.to_path_buf();
        candidate.set_extension(ext.to_string_lossy().trim_start_matches('.'));
        result.push(candidate);
    }

    result
}

#[cfg(not(windows))]
fn candidates(path: &Path) -> Vec<PathBuf> {
    vec![path.to_path_buf()]
}

fn executable(path: &Path) -> bool {
    let metadata = match std::fs::metadata(path) {
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

fn default_path() -> OsString {
    #[cfg(unix)]
    {
        env::join_paths(["/usr/bin", "/bin", "/usr/sbin", "/sbin"])
            .unwrap_or_default()
    }

    #[cfg(windows)]
    {
        env::var_os("PATH").unwrap_or_default()
    }
}

fn find_program(name: &OsStr, use_default_path: bool) -> Option<PathBuf> {
    let path = PathBuf::from(name);

    // Explicit paths: ./program, /usr/bin/program, etc.
    if has_path_separator(name) {
        return candidates(&path)
            .into_iter()
            .find(|candidate| executable(candidate));
    }

    let path_value = if use_default_path {
        default_path()
    } else {
        env::var_os("PATH")?
    };

    for directory in env::split_paths(&path_value) {
        for candidate in candidates(&directory.join(&path)) {
            if executable(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

fn help() {
    println!(
        "command {}\n\
         Execute a command or display information about it.\n\
         \n\
         Usage: command [-p] COMMAND [ARG...]\n\
                command -v COMMAND\n\
                command -V COMMAND\n\
         \n\
         Options:\n\
           -p          Use a default PATH\n\
           -v          Print command path\n\
           -V          Describe command\n\
           -h, --help  Display this help\n\
           --version   Display version information",
        VERSION
    );
}

pub fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);

    let mut mode = 'x';
    let mut use_default_path = false;
    let mut program: Option<OsString> = None;

    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("-h") | Some("--help") => {
                help();
                return ExitCode::SUCCESS;
            }
            Some("-V") => {
                mode = 'V';
            }
            Some("-v") => {
                mode = 'v';
            }
            Some("--version") => {
                println!("command {}", VERSION);
                return ExitCode::SUCCESS;
            }
            Some("-p") => {
                use_default_path = true;
            }
            Some("--") => {
                program = args.next();
                break;
            }
            _ if arg.to_string_lossy().starts_with('-') => {
                eprintln!("command: unknown option: {}", arg.to_string_lossy());
                return ExitCode::from(2);
            }
            _ => {
                program = Some(arg);
                break;
            }
        }
    }

    let program = match program {
        Some(p) => p,
        None => {
            eprintln!("command: missing command");
            return ExitCode::from(2);
        }
    };

    let path = find_program(&program, use_default_path);

    match mode {
        'v' | 'V' => {
            match path {
                Some(path) => {
                    if mode == 'v' {
                        println!("{}", path.display());
                    } else {
                        println!(
                            "{} is {}",
                            program.to_string_lossy(),
                            path.display()
                        );
                    }

                    ExitCode::SUCCESS
                }
                None => {
                    if mode == 'V' {
                        println!("{}: not found", program.to_string_lossy());
                    }

                    ExitCode::from(1)
                }
            }
        }
        _ => {
            let path = match path {
                Some(p) => p,
                None => {
                    eprintln!(
                        "command: {}: not found",
                        program.to_string_lossy()
                    );
                    return ExitCode::from(127);
                }
            };

            match ProcessCommand::new(path).args(args).status() {
                Ok(status) => {
                    ExitCode::from(
                        status.code().unwrap_or(1).clamp(0, 255) as u8
                    )
                }
                Err(error) => {
                    eprintln!("command: {}", error);
                    ExitCode::from(126)
                }
            }
        }
    }
}
