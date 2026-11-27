use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_value(name: &str) -> Option<String> {
    let key = match name {
        "ARG_MAX" => libc::_SC_ARG_MAX,
        "CHILD_MAX" => libc::_SC_CHILD_MAX,
        "CLK_TCK" => libc::_SC_CLK_TCK,
        "OPEN_MAX" => libc::_SC_OPEN_MAX,
        "PAGESIZE" | "PAGE_SIZE" => libc::_SC_PAGESIZE,
        "STREAM_MAX" => libc::_SC_STREAM_MAX,
        "TZNAME_MAX" => libc::_SC_TZNAME_MAX,
        "NPROCESSORS_ONLN" => libc::_SC_NPROCESSORS_ONLN,
        _ => return None,
    };

    let value = unsafe { libc::sysconf(key) };

    if value < 0 {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn get_value(_name: &str) -> Option<String> {
    None
}

pub fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "getconf {}\n\
             Usage: getconf NAME [PATH]\n\
             Query system configuration values.",
            VERSION
        );
        return;
    }

    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("getconf {}", VERSION);
        return;
    }

    if args.is_empty() || args.len() > 2 {
        eprintln!("Usage: getconf NAME [PATH]");
        std::process::exit(2);
    }

    let name = &args[0];

    if args.len() == 2 {
        eprintln!(
            "getconf: PATH-based variables are not supported yet"
        );
        std::process::exit(2);
    }

    match get_value(name) {
        Some(value) => println!("{}", value),
        None => {
            println!("undefined");
            std::process::exit(0);
        }
    }
}
