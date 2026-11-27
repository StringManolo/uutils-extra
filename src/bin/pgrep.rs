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
    list_name: bool,
    list_full: bool,
    count: bool,
    newest: bool,
    oldest: bool,
    delim: Option<String>,
    euid: Vec<u32>,
    uid: Vec<u32>,
    parent: Vec<i32>,
    terminal: Vec<String>,
}

struct M {
    pid: i32,
    comm: String,
    starttime: u64,
}

pub fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut o = Opts::default();
    let mut pattern = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--help" => { print_help(); return ExitCode::SUCCESS; }
            "--version" => { println!("pgrep (uutils-extra) {VERSION}"); return ExitCode::SUCCESS; }
            "-f" | "--full" => o.full = true,
            "-x" | "--exact" => o.exact = true,
            "-v" | "--inverse" => o.inverse = true,
            "-l" | "--list-name" => o.list_name = true,
            "-a" | "--list-full" => o.list_full = true,
            "-c" | "--count" => o.count = true,
            "-n" | "--newest" => o.newest = true,
            "-o" | "--oldest" => o.oldest = true,
            "-d" | "--delimiter" => {
                i += 1;
                if i >= args.len() { eprintln!("pgrep: option requires an argument -- 'd'"); return ExitCode::from(1); }
                o.delim = Some(args[i].clone());
            }
            "-u" | "--euid" => {
                i += 1;
                if i >= args.len() { eprintln!("pgrep: option requires an argument -- 'u'"); return ExitCode::from(1); }
                o.euid = args[i].split(',').filter_map(|s| s.parse().ok()).collect();
            }
            "-U" | "--uid" => {
                i += 1;
                if i >= args.len() { eprintln!("pgrep: option requires an argument -- 'U'"); return ExitCode::from(1); }
                o.uid = args[i].split(',').filter_map(|s| s.parse().ok()).collect();
            }
            "-P" | "--parent" => {
                i += 1;
                if i >= args.len() { eprintln!("pgrep: option requires an argument -- 'P'"); return ExitCode::from(1); }
                o.parent = args[i].split(',').filter_map(|s| s.parse().ok()).collect();
            }
            "-t" | "--terminal" => {
                i += 1;
                if i >= args.len() { eprintln!("pgrep: option requires an argument -- 't'"); return ExitCode::from(1); }
                o.terminal = args[i].split(',').map(|s| s.to_string()).collect();
            }
            s if s.starts_with('-') => {
                eprintln!("pgrep: unrecognized option '{s}'");
                return ExitCode::from(1);
            }
            _ => {
                if pattern.is_some() {
                    eprintln!("pgrep: only one pattern can be provided");
                    return ExitCode::from(1);
                }
                pattern = Some(a.to_string());
            }
        }
        i += 1;
    }
    let pattern = match pattern {
        Some(p) => p,
        None => { eprintln!("pgrep: no matching criteria specified"); return ExitCode::from(1); }
    };
    let pat = if o.exact { format!("^(?:{})$", pattern) } else { pattern };
    let re = match Regex::new(&pat) {
        Ok(r) => r,
        Err(e) => { eprintln!("pgrep: invalid pattern: {e}"); return ExitCode::from(2); }
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
    if o.count {
        println!("{}", matches.len());
        return ExitCode::SUCCESS;
    }
    if o.list_full {
        for m in &matches {
            let cmd = cmdline(m.pid).unwrap_or_else(|| m.comm.clone());
            println!("{} {}", m.pid, cmd);
        }
    } else if o.list_name {
        for m in &matches {
            println!("{} {}", m.pid, m.comm);
        }
    } else {
        let d = o.delim.clone().unwrap_or_else(|| "\n".into());
        let s: Vec<String> = matches.iter().map(|m| m.pid.to_string()).collect();
        println!("{}", s.join(&d));
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
        out.push(M { pid, comm: p.comm, starttime: p.starttime });
    }
    out
}

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

fn print_help() {
    println!("Usage: pgrep [options] <pattern>");
    println!("Look up or signal processes based on name and other attributes.");
    println!();
    println!("  -f, --full            match against full command line");
    println!("  -x, --exact           match exactly");
    println!("  -v, --inverse         invert the match");
    println!("  -l, --list-name       list PID and process name");
    println!("  -a, --list-full       list PID and full command line");
    println!("  -c, --count           count matches");
    println!("  -n, --newest          select only the newest");
    println!("  -o, --oldest          select only the oldest");
    println!("  -d, --delimiter <d>   set the output delimiter");
    println!("  -u, --euid <list>     match effective UIDs");
    println!("  -U, --uid <list>      match real UIDs");
    println!("  -P, --parent <list>   match parent PIDs");
    println!("  -t, --terminal <list> match terminal names");
    println!("      --help            display this help and exit");
    println!("      --version         output version information and exit");
}
