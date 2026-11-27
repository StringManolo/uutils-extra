use std::env;
use std::thread;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const POLL_INTERVAL: Duration = Duration::from_millis(100);

fn help() {
    println!(
        "wait {}\n\
         Wait for process completion.\n\
         \n\
         Usage: wait [OPTION]... [PID...]\n\
         \n\
         Options:\n\
           -n          Wait until any specified process disappears\n\
           -h, --help  Display this help\n\
           -V, --version Display version information\n\
         \n\
         Without PID arguments, exits successfully.\n\
         Note: An external wait cannot wait for child processes\n\
         belonging to its parent shell.",
        VERSION
    );
}

#[cfg(unix)]
fn process_exists(pid: i32) -> Result<bool, std::io::Error> {
    let result = unsafe { libc::kill(pid, 0) };

    if result == 0 {
        return Ok(true);
    }

    let error = std::io::Error::last_os_error();

    match error.raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        Some(libc::EPERM) => Ok(true),
        _ => Err(error),
    }
}

#[cfg(windows)]
fn process_exists(pid: i32) -> Result<bool, std::io::Error> {
    use std::os::windows::io::FromRawHandle;
    use std::ptr;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const WAIT_TIMEOUT: u32 = 0x00000102;
    const WAIT_OBJECT_0: u32 = 0x00000000;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(
            access: u32,
            inherit: i32,
            pid: u32,
        ) -> *mut std::ffi::c_void;

        fn WaitForSingleObject(
            handle: *mut std::ffi::c_void,
            milliseconds: u32,
        ) -> u32;

        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }

    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid as u32,
        )
    };

    if handle.is_null() {
        return Ok(false);
    }

    let result = unsafe { WaitForSingleObject(handle, 0) };

    unsafe {
        CloseHandle(handle);
    }

    match result {
        WAIT_TIMEOUT => Ok(true),
        WAIT_OBJECT_0 => Ok(false),
        _ => Err(std::io::Error::last_os_error()),
    }
}

#[cfg(not(any(unix, windows)))]
fn process_exists(_pid: i32) -> Result<bool, std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "process checking is not supported on this platform",
    ))
}

fn wait_for_pid(pid: i32) -> i32 {
    if pid <= 0 {
        eprintln!("wait: invalid PID: {}", pid);
        return 2;
    }

    loop {
        match process_exists(pid) {
            Ok(true) => {
                thread::sleep(POLL_INTERVAL);
            }
            Ok(false) => {
                return 0;
            }
            Err(error) => {
                eprintln!("wait: PID {}: {}", pid, error);
                return 127;
            }
        }
    }
}

fn wait_any(pids: &[i32]) -> i32 {
    loop {
        for &pid in pids {
            match process_exists(pid) {
                Ok(false) => return 0,
                Ok(true) => {}
                Err(error) => {
                    eprintln!("wait: PID {}: {}", pid, error);
                    return 127;
                }
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn parse_pid(value: &str) -> Result<i32, String> {
    let pid = value
        .parse::<i32>()
        .map_err(|_| format!("invalid PID: {}", value))?;

    if pid <= 0 {
        return Err(format!("invalid PID: {}", value));
    }

    Ok(pid)
}

pub fn main() {
    let mut args = env::args().skip(1);
    let mut any = false;
    let mut pids = Vec::new();
    let mut options = true;

    while let Some(arg) = args.next() {
        if options {
            match arg.as_str() {
                "--" => {
                    options = false;
                    continue;
                }
                "-h" | "--help" => {
                    help();
                    return;
                }
                "-V" | "--version" => {
                    println!("wait {}", VERSION);
                    return;
                }
                "-n" => {
                    any = true;
                    continue;
                }
                _ if arg.starts_with('-') => {
                    eprintln!("wait: unknown option: {}", arg);
                    std::process::exit(2);
                }
                _ => {}
            }
        }

        match parse_pid(&arg) {
            Ok(pid) => pids.push(pid),
            Err(error) => {
                eprintln!("wait: {}", error);
                std::process::exit(2);
            }
        }
    }

    // An external process cannot wait for its parent shell's children.
    if pids.is_empty() {
        return;
    }

    let exit_code = if any {
        wait_any(&pids)
    } else {
        let mut result = 0;

        for pid in pids {
            let code = wait_for_pid(pid);

            if code != 0 {
                result = code;
            }
        }

        result
    };

    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_pid() {
        assert_eq!(parse_pid("123"), Ok(123));
    }

    #[test]
    fn rejects_zero_pid() {
        assert!(parse_pid("0").is_err());
    }

    #[test]
    fn rejects_negative_pid() {
        assert!(parse_pid("-123").is_err());
    }

    #[test]
    fn rejects_invalid_pid() {
        assert!(parse_pid("abc").is_err());
    }

    #[test]
    fn current_process_exists() {
        assert_eq!(process_exists(std::process::id() as i32), Ok(true));
    }

    #[test]
    fn invalid_pid_does_not_exist() {
        assert_eq!(process_exists(0), Ok(false));
    }
}
