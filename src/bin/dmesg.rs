use std::env;
use std::fs;
use std::io::Write;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const SYSLOG_ACTION_READ: i32 = 2;
const SYSLOG_ACTION_READ_ALL: i32 = 3;
const SYSLOG_ACTION_READ_CLEAR: i32 = 4;
const SYSLOG_ACTION_CLEAR: i32 = 5;
const SYSLOG_ACTION_CONSOLE_OFF: i32 = 6;
const SYSLOG_ACTION_CONSOLE_ON: i32 = 7;
const SYSLOG_ACTION_CONSOLE_LEVEL: i32 = 8;
const SYSLOG_ACTION_SIZE_UNREAD: i32 = 9;

struct Opts {
    ctime: bool,
    notime: bool,
    decode: bool,
    human: bool,
    follow: bool,
    read_clear: bool,
    clear: bool,
    console_level: Option<u8>,
    console_off: bool,
    console_on: bool,
    levels: Vec<u8>,
    facilities: Vec<u8>,
}

pub fn main() -> ExitCode {
    let mut o = Opts {
        ctime: false, notime: false, decode: false, human: false,
        follow: false, read_clear: false, clear: false,
        console_level: None, console_off: false, console_on: false,
        levels: Vec::new(), facilities: Vec::new(),
    };
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--help" => { print_help(); return ExitCode::SUCCESS; }
            "--version" => { println!("dmesg (uutils-extra) {VERSION}"); return ExitCode::SUCCESS; }
            "-T" | "--ctime" => o.ctime = true,
            "-t" | "--notime" => o.notime = true,
            "-x" | "--decode" => o.decode = true,
            "-H" | "--human" => o.human = true,
            "-w" | "--follow" => o.follow = true,
            "-c" | "--read-clear" => o.read_clear = true,
            "-C" | "--clear" => o.clear = true,
            "-D" | "--console-off" => o.console_off = true,
            "-E" | "--console-on" => o.console_on = true,
            "-l" | "--level" => {
                i += 1;
                if i >= args.len() { eprintln!("dmesg: option requires an argument -- 'l'"); return ExitCode::from(1); }
                o.levels = args[i].split(',').filter_map(parse_level).collect();
            }
            "-f" | "--facility" => {
                i += 1;
                if i >= args.len() { eprintln!("dmesg: option requires an argument -- 'f'"); return ExitCode::from(1); }
                o.facilities = args[i].split(',').filter_map(parse_facility).collect();
            }
            "-n" | "--console-level" => {
                i += 1;
                if i >= args.len() { eprintln!("dmesg: option requires an argument -- 'n'"); return ExitCode::from(1); }
                match parse_level(&args[i]) {
                    Some(v) => o.console_level = Some(v),
                    None => { eprintln!("dmesg: unknown level '{}'", args[i]); return ExitCode::from(1); }
                }
            }
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("dmesg: unrecognized option '{s}'");
                return ExitCode::from(1);
            }
            _ => {}
        }
        i += 1;
    }
    if let Err(e) = run(o) {
        eprintln!("dmesg: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run(o: Opts) -> std::io::Result<()> {
    if o.clear { syslog_call(SYSLOG_ACTION_CLEAR, 0)?; return Ok(()); }
    if let Some(lvl) = o.console_level { syslog_call(SYSLOG_ACTION_CONSOLE_LEVEL, lvl as i32)?; }
    if o.console_off { syslog_call(SYSLOG_ACTION_CONSOLE_OFF, 0)?; }
    if o.console_on  { syslog_call(SYSLOG_ACTION_CONSOLE_ON, 0)?; }
    let boot = boot_time_secs();
    if o.follow {
        loop {
            let mut buf = vec![0u8; 8192];
            let n = syslog_read(SYSLOG_ACTION_READ, &mut buf)?;
            if n > 0 { buf.truncate(n); process_buf(&buf, &o, boot)?; }
        }
    } else {
        let action = if o.read_clear { SYSLOG_ACTION_READ_CLEAR } else { SYSLOG_ACTION_READ_ALL };
        let size = syslog_size(SYSLOG_ACTION_SIZE_UNREAD)?;
        let mut buf = vec![0u8; size.max(1)];
        let n = syslog_read(action, &mut buf)?;
        buf.truncate(n);
        process_buf(&buf, &o, boot)?;
    }
    Ok(())
}

fn syslog_read(action: i32, buf: &mut [u8]) -> std::io::Result<usize> {
    let n = unsafe { libc::syscall(libc::SYS_syslog, action, buf.as_mut_ptr() as *mut i8, buf.len() as i32) };
    if n < 0 { return Err(std::io::Error::last_os_error()); }
    Ok(n as usize)
}

fn syslog_call(action: i32, len: i32) -> std::io::Result<()> {
    let n = unsafe { libc::syscall(libc::SYS_syslog, action, std::ptr::null::<i8>(), len) };
    if n < 0 { return Err(std::io::Error::last_os_error()); }
    Ok(())
}

fn syslog_size(action: i32) -> std::io::Result<usize> {
    let n = unsafe { libc::syscall(libc::SYS_syslog, action, std::ptr::null::<i8>(), 0) };
    if n < 0 { return Err(std::io::Error::last_os_error()); }
    Ok(n as usize)
}

fn process_buf(buf: &[u8], o: &Opts, boot: i64) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in buf.split(|&b| b == b'\n') {
        if line.is_empty() { continue; }
        if let Some((pri, ts, usec, msg)) = parse_record(line) {
            let level = pri & 7;
            let fac = pri >> 3;
            if !o.levels.is_empty() && !o.levels.contains(&level) { continue; }
            if !o.facilities.is_empty() && !o.facilities.contains(&fac) { continue; }
            write_record(&mut out, o, boot, ts, usec, level, fac, msg)?;
        }
    }
    out.flush()
}

fn parse_record(line: &[u8]) -> Option<(u8, i64, i64, &str)> {
    let close = line.iter().position(|&b| b == b'>')?;
    let pri: u8 = std::str::from_utf8(&line[1..close]).ok()?.parse().ok()?;
    let rest = &line[close + 1..];
    let semi = rest.iter().position(|&b| b == b';')?;
    let header = std::str::from_utf8(&rest[..semi]).ok()?;
    let msg = std::str::from_utf8(&rest[semi + 1..]).ok()?;
    let mut parts = header.split(',');
    let ts: i64 = parts.next()?.parse().ok()?;
    let usec: i64 = parts.next()?.parse().ok()?;
    let _flag = parts.next()?;
    Some((pri, ts, usec, msg))
}

fn write_record(out: &mut impl Write, o: &Opts, boot: i64, ts: i64, usec: i64, level: u8, fac: u8, msg: &str) -> std::io::Result<()> {
    let prefix = if o.notime {
        String::new()
    } else if o.ctime {
        format!("[{}] ", format_ctime(boot + ts))
    } else {
        format!("[{:5}.{:06}] ", ts, usec)
    };
    let decoded = if o.decode {
        format!("{:<5}:{:<7}: ", facility_name(fac), level_name(level))
    } else {
        String::new()
    };
    if o.human {
        writeln!(out, "\x1b[{}m{}{}\x1b[0m{}", level_color(level), decoded, prefix, msg)
    } else {
        writeln!(out, "{}{}{}", decoded, prefix, msg)
    }
}

fn format_ctime(secs: i64) -> String {
    let mut t = secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&mut t, &mut tm); }
    let wday = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"][(tm.tm_wday as usize) % 7];
    let mon  = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"][(tm.tm_mon as usize) % 12];
    format!("{} {} {:2} {:02}:{:02}:{:02} {}",
        wday, mon, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec, tm.tm_year + 1900)
}

fn boot_time_secs() -> i64 {
    let now = unsafe { libc::time(std::ptr::null_mut()) } as i64;
    let uptime = fs::read_to_string("/proc/uptime").ok()
        .and_then(|s| s.split_whitespace().next().and_then(|x| x.parse::<f64>().ok()))
        .unwrap_or(0.0);
    now - uptime as i64
}

fn facility_name(f: u8) -> &'static str {
    match f {
        0 => "kern", 1 => "user", 2 => "mail", 3 => "daemon", 4 => "auth",
        5 => "syslog", 6 => "lpr", 7 => "news", 8 => "uucp", 9 => "cron",
        10 => "authpriv", 11 => "ftp", 16 => "local0", 17 => "local1",
        18 => "local2", 19 => "local3", 20 => "local4", 21 => "local5",
        22 => "local6", 23 => "local7", _ => "unknown",
    }
}

fn level_name(l: u8) -> &'static str {
    match l {
        0 => "emerg", 1 => "alert", 2 => "crit", 3 => "err",
        4 => "warn", 5 => "notice", 6 => "info", 7 => "debug", _ => "unknown",
    }
}

fn level_color(l: u8) -> &'static str {
    match l {
        0..=3 => "31", 4 => "33", 5 => "36", 6 => "32", _ => "0",
    }
}

fn parse_level(s: &str) -> Option<u8> {
    match s {
        "emerg" => Some(0), "alert" => Some(1), "crit" => Some(2), "err" => Some(3),
        "warn" => Some(4), "notice" => Some(5), "info" => Some(6), "debug" => Some(7),
        _ => s.parse().ok().filter(|&x| x < 8),
    }
}

fn parse_facility(s: &str) -> Option<u8> {
    match s {
        "kern" => Some(0), "user" => Some(1), "mail" => Some(2), "daemon" => Some(3),
        "auth" => Some(4), "syslog" => Some(5), "lpr" => Some(6), "news" => Some(7),
        "uucp" => Some(8), "cron" => Some(9), "authpriv" => Some(10), "ftp" => Some(11),
        "local0" => Some(16), "local1" => Some(17), "local2" => Some(18), "local3" => Some(19),
        "local4" => Some(20), "local5" => Some(21), "local6" => Some(22), "local7" => Some(23),
        _ => None,
    }
}

fn print_help() {
    println!("Usage: dmesg [options]");
    println!("Print or control the kernel ring buffer.");
    println!();
    println!("  -T, --ctime              show human-readable timestamps");
    println!("  -t, --notime             don't print timestamps");
    println!("  -x, --decode             decode facility and level to strings");
    println!("  -H, --human              human-readable output with colors");
    println!("  -l, --level <list>       restrict to levels");
    println!("  -f, --facility <list>    restrict to facilities");
    println!("  -w, --follow             wait for new messages");
    println!("  -c, --read-clear         read and clear ring buffer");
    println!("  -C, --clear              clear ring buffer");
    println!("  -n, --console-level <l>  set console logging level");
    println!("  -D, --console-off        disable console printing");
    println!("  -E, --console-on         enable console printing");
    println!("      --help               display this help and exit");
    println!("      --version            output version information and exit");
}
