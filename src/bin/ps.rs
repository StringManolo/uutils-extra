use std::env;
use std::fs;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Proc {
    pid: i32,
    comm: String,
    state: char,
    ppid: i32,
    tty_nr: i64,
    utime: u64,
    stime: u64,
    starttime: u64,
    vsize: u64,
    rss: u64,
}

pub fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let (mut all, mut full, mut bsd, mut unix_fmt) = (false, false, false, false);
    let mut pid_filter: Option<Vec<i32>> = None;
    let mut ppid_filter: Option<i32> = None;
    let mut user_filter: Option<String> = None;
    let mut format: Option<Vec<String>> = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--help" => { print_help(); return ExitCode::SUCCESS; }
            "--version" => { println!("ps (uutils-extra) {VERSION}"); return ExitCode::SUCCESS; }
            "-e" | "-A" | "--all" => all = true,
            "-f" => full = true,
            "aux" | "-aux" => { bsd = true; all = true; }
            "-ef" => { unix_fmt = true; all = true; }
            "-p" | "--pid" => {
                i += 1;
                if i >= args.len() { eprintln!("ps: option requires an argument -- 'p'"); return ExitCode::from(1); }
                pid_filter = Some(args[i].split(',').filter_map(|s| s.parse().ok()).collect());
            }
            "-P" | "--ppid" => {
                i += 1;
                if i >= args.len() { eprintln!("ps: option requires an argument -- 'P'"); return ExitCode::from(1); }
                ppid_filter = args[i].parse().ok();
            }
            "-u" | "--user" => {
                i += 1;
                if i >= args.len() { eprintln!("ps: option requires an argument -- 'u'"); return ExitCode::from(1); }
                user_filter = Some(args[i].clone());
            }
            "-o" => {
                i += 1;
                if i >= args.len() { eprintln!("ps: option requires an argument -- 'o'"); return ExitCode::from(1); }
                format = Some(args[i].split(',').map(|s| s.trim().to_string()).collect());
            }
            s if s.starts_with('-') => {
                eprintln!("ps: unrecognized option '{s}'");
                return ExitCode::from(1);
            }
            _ => {}
        }
        i += 1;
    }
    let procs = match collect_procs() { Ok(p) => p, Err(e) => { eprintln!("ps: {e}"); return ExitCode::from(1); } };
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
    let uptime = uptime_secs();
    let mem_total = mem_total_kb() * 1024;
    let boot = boot_time_secs();
    let my_pid = std::process::id() as i32;
    let my_tty = current_tty_nr();

    let mut selected: Vec<&Proc> = procs.iter().filter(|p| {
        if let Some(ref v) = pid_filter { if !v.contains(&p.pid) { return false; } }
        if let Some(pp) = ppid_filter { if p.ppid != pp { return false; } }
        if let Some(ref u) = user_filter { if uid_name(p.pid) != *u { return false; } }
        if !all && pid_filter.is_none() && p.tty_nr != my_tty && p.pid != my_pid { return false; }
        true
    }).collect();
    selected.sort_by_key(|p| p.pid);

    if bsd {
        print_bsd(&selected, page_size, hz, uptime, mem_total, boot);
    } else if unix_fmt || full {
        print_full(&selected, hz, boot);
    } else if let Some(fmt) = format {
        print_custom(&selected, &fmt, page_size, hz, uptime, mem_total, boot);
    } else {
        print_default(&selected, hz);
    }
    ExitCode::SUCCESS
}

fn collect_procs() -> std::io::Result<Vec<Proc>> {
    let mut out = Vec::new();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let name = entry.file_name();
        let s = name.to_string_lossy();
        if let Ok(pid) = s.parse::<i32>() {
            if let Ok(p) = read_proc(pid) { out.push(p); }
        }
    }
    Ok(out)
}

fn read_proc(pid: i32) -> std::io::Result<Proc> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    parse_stat(&stat).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad stat"))
}

fn parse_stat(s: &str) -> Option<Proc> {
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    let pid: i32 = s[..open].trim().parse().ok()?;
    let comm = s[open + 1..close].to_string();
    let rest = &s[close + 2..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    if f.len() < 22 { return None; }
    Some(Proc {
        pid,
        comm,
        state: f[0].chars().next()?,
        ppid: f[1].parse().ok()?,
        tty_nr: f[4].parse().ok()?,
        utime: f[11].parse().ok()?,
        stime: f[12].parse().ok()?,
        starttime: f[19].parse().ok()?,
        vsize: f[20].parse().ok()?,
        rss: f[21].parse().ok()?,
    })
}

fn cmdline(pid: i32) -> Option<String> {
    let data = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    if data.is_empty() { return None; }
    let s = String::from_utf8_lossy(&data).replace('\0', " ");
    Some(s.trim_end().to_string())
}

fn uid_of(pid: i32) -> Option<u32> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

fn uid_name(pid: i32) -> String {
    let uid = uid_of(pid).unwrap_or(0);
    if let Ok(s) = fs::read_to_string("/etc/passwd") {
        for line in s.lines() {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() >= 3 && f[2].parse::<u32>().ok() == Some(uid) {
                return f[0].to_string();
            }
        }
    }
    uid.to_string()
}

fn current_tty_nr() -> i64 {
    fs::read_to_string("/proc/self/stat").ok()
        .and_then(|s| parse_stat(&s)).map(|p| p.tty_nr).unwrap_or(0)
}

fn uptime_secs() -> u64 {
    fs::read_to_string("/proc/uptime").ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .map(|x| x as u64).unwrap_or(0)
}

fn mem_total_kb() -> u64 {
    fs::read_to_string("/proc/meminfo").ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1)?.parse().ok()))
        .unwrap_or(0)
}

fn boot_time_secs() -> i64 {
    let now = unsafe { libc::time(std::ptr::null_mut()) } as i64;
    now - uptime_secs() as i64
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

fn format_time(ticks: u64, hz: u64) -> String {
    let total = ticks / hz.max(1);
    let ss = total % 60;
    let mm = (total / 60) % 60;
    let hh = (total / 3600) % 24;
    let dd = total / 86400;
    if dd > 0 { format!("{}-{:02}:{:02}:{:02}", dd, hh, mm, ss) }
    else if hh > 0 { format!("{:02}:{:02}:{:02}", hh, mm, ss) }
    else { format!("{:02}:{:02}", mm, ss) }
}

fn format_start(starttime: u64, hz: u64, boot: i64) -> String {
    let secs = boot + (starttime / hz.max(1)) as i64;
    let mut t = secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&mut t, &mut tm); }
    let now = unsafe { libc::time(std::ptr::null_mut()) } as libc::time_t;
    let mut now_t = now;
    let mut now_tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&mut now_t, &mut now_tm); }
    if tm.tm_year == now_tm.tm_year && tm.tm_yday == now_tm.tm_yday {
        format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
    } else if tm.tm_year == now_tm.tm_year {
        let mon = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"][(tm.tm_mon as usize) % 12];
        format!("{}{:02}", mon, tm.tm_mday)
    } else {
        format!("{:04}", tm.tm_year + 1900)
    }
}

fn print_default(procs: &[&Proc], hz: u64) {
    println!("    PID TTY          TIME CMD");
    for p in procs {
        let time = format_time(p.utime + p.stime, hz);
        println!("{:7} {:<8} {:>8} {}", p.pid, format_tty(p.tty_nr), time, p.comm);
    }
}

fn print_full(procs: &[&Proc], hz: u64, boot: i64) {
    println!("UID        PID  PPID  C STIME TTY          TIME CMD");
    for p in procs {
        let uid = uid_name(p.pid);
        let c = 0;
        let stime = format_start(p.starttime, hz, boot);
        let time = format_time(p.utime + p.stime, hz);
        let cmd = cmdline(p.pid).unwrap_or_else(|| format!("[{}]", p.comm));
        println!("{:<8} {:5} {:5} {:2} {:<5} {:<12} {:>8} {}",
            uid, p.pid, p.ppid, c, stime, format_tty(p.tty_nr), time, cmd);
    }
}

fn print_bsd(procs: &[&Proc], page_size: u64, hz: u64, uptime: u64, mem_total: u64, boot: i64) {
    println!("USER       PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND");
    for p in procs {
        let user = uid_name(p.pid);
        let cpu = if uptime > 0 { (p.utime + p.stime) as f64 / (uptime * hz) as f64 * 100.0 } else { 0.0 };
        let mem = if mem_total > 0 { (p.rss * page_size) as f64 / mem_total as f64 * 100.0 } else { 0.0 };
        let vsz = p.vsize / 1024;
        let rss = p.rss * page_size / 1024;
        let start = format_start(p.starttime, hz, boot);
        let time = format_time(p.utime + p.stime, hz);
        let cmd = cmdline(p.pid).unwrap_or_else(|| format!("[{}]", p.comm));
        println!("{:<8} {:5} {:4.1} {:4.1} {:6} {:5} {:<8} {:<4} {:<5} {:>8} {}",
            user, p.pid, cpu, mem, vsz, rss, format_tty(p.tty_nr), p.state, start, time, cmd);
    }
}

fn print_custom(procs: &[&Proc], fmt: &[String], page_size: u64, hz: u64, uptime: u64, mem_total: u64, boot: i64) {
    let headers: Vec<String> = fmt.iter().map(|f| field_header(f)).collect();
    println!("{}", headers.join(" "));
    for p in procs {
        let fields: Vec<String> = fmt.iter().map(|f| field_value(p, f, page_size, hz, uptime, mem_total, boot)).collect();
        println!("{}", fields.join(" "));
    }
}

fn field_header(f: &str) -> String {
    match f {
        "pid" => "PID".into(), "ppid" => "PPID".into(), "comm" => "COMMAND".into(),
        "cmd" | "args" => "COMMAND".into(), "user" | "uid" => "USER".into(),
        "tty" => "TT".into(), "stat" => "STAT".into(), "time" => "TIME".into(),
        "etime" => "ELAPSED".into(), "%cpu" => "%CPU".into(), "%mem" => "%MEM".into(),
        "vsz" => "VSZ".into(), "rss" => "RSS".into(), "start" | "stime" => "START".into(),
        _ => f.to_uppercase(),
    }
}

fn field_value(p: &Proc, f: &str, page_size: u64, hz: u64, uptime: u64, mem_total: u64, boot: i64) -> String {
    match f {
        "pid" => p.pid.to_string(),
        "ppid" => p.ppid.to_string(),
        "comm" => p.comm.clone(),
        "cmd" | "args" => cmdline(p.pid).unwrap_or_else(|| format!("[{}]", p.comm)),
        "user" | "uid" => uid_name(p.pid),
        "tty" => format_tty(p.tty_nr),
        "stat" => p.state.to_string(),
        "time" => format_time(p.utime + p.stime, hz),
        "etime" => format_time(uptime.saturating_sub(p.starttime / hz.max(1)), 1),
        "%cpu" => {
            let v = if uptime > 0 { (p.utime + p.stime) as f64 / (uptime * hz) as f64 * 100.0 } else { 0.0 };
            format!("{v:.1}")
        }
        "%mem" => {
            let v = if mem_total > 0 { (p.rss * page_size) as f64 / mem_total as f64 * 100.0 } else { 0.0 };
            format!("{v:.1}")
        }
        "vsz" => (p.vsize / 1024).to_string(),
        "rss" => (p.rss * page_size / 1024).to_string(),
        "start" | "stime" => format_start(p.starttime, hz, boot),
        _ => "-".into(),
    }
}

fn print_help() {
    println!("Usage: ps [options]");
    println!("Report a snapshot of current processes.");
    println!();
    println!("  -e, -A, --all      select all processes");
    println!("  -f                 full-format listing");
    println!("  -ef                UNIX full-format listing");
    println!("  aux                BSD format, all processes");
    println!("  -p, --pid <list>   select by PID");
    println!("  -P, --ppid <pid>   select by parent PID");
    println!("  -u, --user <user>  select by user");
    println!("  -o <format>        custom output format (pid,ppid,comm,cmd,user,");
    println!("                     tty,stat,time,etime,%cpu,%mem,vsz,rss,start)");
    println!("      --help         display this help and exit");
    println!("      --version      output version information and exit");
}
