use std::env;
use std::fs;
use std::process::ExitCode;

use regex::Regex;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct Opts {
    full: bool,
    exact: bool,
    inverse: bool,
    newest: bool,
    oldest: bool,
    euid: Vec<u32>,
    uid: Vec<u32>,
    parent: Vec<i32>,
    terminal: Vec<String>,
    signal: i32,
}

pub fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut o = Opts { signal: libc::SIGTERM, ..Default::default() };
    let mut pattern = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--help" => { print_help(); return ExitCode::SUCCESS; }
            "--version" => { println!("pkill (uutils-extra) {VERSION}"); return ExitCode::SUCCESS; }
            "-f" | "--full" => o.full = true,
            "-x" | "--exact" => o.exact = true,
            "-v" | "--inverse" => o.inverse = true,
            "-n" | "--newest" => o.newest = true,
            "-o" | "--oldest" => o.oldest = true,
            "-u" | "--euid" => {
                i += 1;
                if i >= args.len() { eprintln!("pkill: option requires an argument -- 'u'"); return ExitCode::from(1); }
                o.euid = args[i].split(',').filter_map(|s| s.parse().ok()).collect();
            }
            "-U" | "--uid" => {
                i += 1;
                if i >= args.len() { eprintln!("pkill: option requires an argument -- 'U'"); return ExitCode::from(1); }
                o.uid = args[i].split(',').filter_map(|s| s.parse().ok()).collect();
            }
            "-P" | "--parent" => {
                i += 1;
                if i >= args.len() { eprintln!("pkill: option requires an argument -- 'P'"); return ExitCode::from(1); }
                o.parent = args[i].split(',').filter_map(|s| s.parse().ok()).collect();
            }
            "-t" | "--terminal" => {
                i += 1;
                if i >= args.len() { eprintln!("pkill: option requires an argument -- 't'"); return ExitCode::from(1); }
                o.terminal = args[i].split(',').map(|s| s.to_string()).collect();
            }
            "--signal" => {
                i += 1;
                if i >= args.len() { eprintln!("pkill: option requires an argument -- 'signal'"); return ExitCode::from(1); }
                match parse_signal(&args[i]) {
                    Some(s) => o.signal = s,
                    None => { eprintln!("pkill: unknown signal '{}'", args[i]); return ExitCode::from(1); }
                }
            }
            s if s.starts_with('-') && s.len() > 1 => {
                match parse_signal(&s[1..]) {
                    Some(sig) => o.signal = sig,
                    None => { eprintln!("pkill: unrecognized option '{s}'"); return ExitCode::from(1); }
                }
            }
            _ => {
                if pattern.is_some() {
                    eprintln!("pkill: only one pattern can be provided");
                    return ExitCode::from(1);
                }
                pattern = Some(a.to_string());
            }
        }
        i += 1;
    }
    let pattern = match pattern {
        Some(p) => p,
        None => { eprintln!("pkill: no matching criteria specified"); return ExitCode::from(1); }
    };
    let pat = if o.exact { format!("^(?:{})$", pattern) } else { pattern };
    let re = match Regex::new(&pat) {
        Ok(r) => r,
        Err(e) => { eprintln!("pkill: invalid pattern: {e}"); return ExitCode::from(2); }
    };
    let mut matches = find_matches(&o, &re);
    if matches.is_empty() { return ExitCode::from(1); }
    if o.newest {
        matches.sort_by_key(|m| m.starttime);
        let last = matches.last().unwrap().pid;
        matches.retain(|m| m.pid == last);
    } else if o.oldest {
        matches.sort_by_key(|m| m.starttime);
        let first = matches[0].pid;
        matches.retain(|m| m.pid == first);
    }
    for m in &matches {
        unsafe { libc::kill(m.pid, o.signal); }
    }
    ExitCode::SUCCESS
}

fn find_matches(o: &Opts, re: &Regex) -> Vec<M> {
    let mut out = Vec::new();
    let my_pid = std::process::id() as i32;
    let entries = match fs::read_dir("/proc") { Ok(e) => e, Err(_) => return out };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let s = name.to_string_lossy();
        let pid: i32 = match s.parse() { Ok(p) => p, Err(_) => continue };
        if pid == my_pid { continue; }
        let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) { Ok(s) => s, Err(_) => continue };
        let p = match parse_stat(&stat) { Some(p) => p, None => continue };
        if !o.uid.is_empty() {
            let u = match uid_of(pid, false) { Some(u) => u, None => continue };
            if !o.uid.contains(&u) { continue; }
        }
        if !o.euid.is_empty() {
            let u = match uid_of(pid, true) { Some(u) => u, None => continue };
            if !o.euid.contains(&u) { continue; }
        }
        if !o.parent.is_empty() && !o.parent.contains(&p.ppid) { continue; }
        if !o.terminal.is_empty() {
            let tty = format_tty(p.tty_nr);
            if !o.terminal.iter().any(|t| t == &tty) { continue; }
        }
        let hay = if o.full {
            cmdline(pid).unwrap_or_else(|| p.comm.clone())
        } else {
            p.comm.clone()
        };
        let m = re.is_match(&hay);
        if o.inverse == m { continue; }
        out.push(M { pid, starttime: p.starttime });
    }
    out
}

struct M { pid: i32, starttime: u64 }
struct S { comm: String, ppid: i32, tty_nr: i64, starttime: u64 }

fn parse_stat(s: &str) -> Option<S> {
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    let comm = s[open + 1..close].to_string();
    let f: Vec<&str> = s[close + 2..].split_whitespace().collect();
    if f.len() < 22 { return None; }
    Some(S { comm, ppid: f[1].parse().ok()?, tty_nr: f[4].parse().ok()?, starttime: f[19].parse().ok()? })
}

fn cmdline(pid: i32) -> Option<String> {
    let data = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    if data.is_empty() { return None; }
    let s = String::from_utf8_lossy(&data).replace('\0', " ");
    Some(s.trim_end().to_string())
}

fn uid_of(pid: i32, effective: bool) -> Option<u32> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let mut it = rest.split_whitespace();
            let real = it.next()?.parse().ok();
            let eff = it.next().and_then(|s| s.parse().ok());
            return if effective { eff } else { real };
        }
    }
    None
}

fn format_tty(tty_nr: i64) -> String {
    if tty_nr == 0 { return "?".into(); }
    let dev = tty_nr as u32;
    let major = (dev >> 8) & 0xfff;
    let minor = (dev & 0xff) | ((dev >> 12) & 0xfff00);
    if (136..=143).contains(&major) {
        format!("pts/{}", (major - 136) * 256 + minor)
    } else if major == 4 {
        format!("tty{}", minor)
    } else if major == 5 {
        "tty".into()
    } else {
        format!("{}:{}", major, minor)
    }
}

fn parse_signal(s: &str) -> Option<i32> {
    let s = s.strip_prefix("SIG").unwrap_or(s);
    let s = s.to_uppercase();
    match s.as_str() {
        "TERM" => Some(libc::SIGTERM), "KILL" => Some(libc::SIGKILL), "INT" => Some(libc::SIGINT),
        "HUP" => Some(libc::SIGHUP), "QUIT" => Some(libc::SIGQUIT), "USR1" => Some(libc::SIGUSR1),
        "USR2" => Some(libc::SIGUSR2), "STOP" => Some(libc::SIGSTOP), "CONT" => Some(libc::SIGCONT),
        "TSTP" => Some(libc::SIGTSTP), "TTIN" => Some(libc::SIGTTIN), "TTOU" => Some(libc::SIGTTOU),
        "CHLD" => Some(libc::SIGCHLD), "ALRM" => Some(libc::SIGALRM), "PIPE" => Some(libc::SIGPIPE),
        "SEGV" => Some(libc::SIGSEGV), "ABRT" => Some(libc::SIGABRT), "FPE" => Some(libc::SIGFPE),
        "ILL" => Some(libc::SIGILL), "BUS" => Some(libc::SIGBUS), "SYS" => Some(libc::SIGSYS),
        "TRAP" => Some(libc::SIGTRAP), "URG" => Some(libc::SIGURG), "XCPU" => Some(libc::SIGXCPU),
        "XFSZ" => Some(libc::SIGXFSZ), "VTALRM" => Some(libc::SIGVTALRM), "PROF" => Some(libc::SIGPROF),
        "WINCH" => Some(libc::SIGWINCH), "IO" => Some(libc::SIGIO), "PWR" => Some(libc::SIGPWR),
        _ => s.parse().ok(),
    }
}

fn print_help() {
    println!("Usage: pkill [options] <pattern>");
    println!("Signal processes based on name and other attributes.");
    println!();
    println!("  -<sig>, --signal <sig>  signal to send (default TERM)");
    println!("  -f, --full              match against full command line");
    println!("  -x, --exact             match exactly");
    println!("  -v, --inverse           invert the match");
    println!("  -n, --newest            select only the newest");
    println!("  -o, --oldest            select only the oldest");
    println!("  -u, --euid <list>       match effective UIDs");
    println!("  -U, --uid <list>        match real UIDs");
    println!("  -P, --parent <list>     match parent PIDs");
    println!("  -t, --terminal <list>   match terminal names");
    println!("      --help              display this help and exit");
    println!("      --version           output version information and exit");
}
