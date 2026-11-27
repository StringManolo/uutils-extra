// Multicall dispatcher: symlink this binary as `grep`, `sed`, etc.,
// or invoke it as `multicall <cmd> [args...]`.

mod clear    { include!("clear.rs"); }
mod command  { include!("command.rs"); }
mod file     { include!("file.rs"); }
mod find     { include!("find.rs"); }
mod getconf  { include!("getconf.rs"); }
mod grep     { include!("grep.rs"); }
mod sed      { include!("sed.rs"); }
mod wait     { include!("wait.rs"); }
mod which    { include!("which.rs"); }
mod ldd      { include!("ldd.rs"); }
mod dmesg    { include!("dmesg.rs"); }
mod ps       { include!("ps.rs"); }
mod pgrep    { include!("pgrep.rs"); }
mod pkill    { include!("pkill.rs"); }

use std::env;
use std::os::unix::process::CommandExt;
use std::path::Path;

const KNOWN: &[&str] = &[
    "clear", "command", "file", "find", "getconf", "grep",
    "sed", "wait", "which", "ldd", "dmesg", "ps", "pgrep", "pkill",
];

const MULTICALL_NAMES: &[&str] = &["uutils-extra", "uutils-extra-multicall"];

fn basename(p: &str) -> String {
    Path::new(p)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(p)
        .to_string()
}

pub fn main() {
    let argv: Vec<String> = env::args().collect();
    let prog = basename(&argv[0]);

    // 1) Symlink mode: invoked directly as a known command.
    if is_known(&prog) {
        dispatch(&prog);
    }

    // 2) Multicall mode: `uutils-extra-multicall <cmd> [args...]`, `--list`, `--install`.
    if argv.len() < 2 {
        if MULTICALL_NAMES.contains(&prog.as_str()) {
            usage(&prog);
            std::process::exit(1);
        }
        eprintln!("{}: unknown command", prog);
        std::process::exit(127);
    }

    match argv[1].as_str() {
        "--help" | "-h" => { usage(&prog); return; }
        "--version" | "-V" => { println!("{} {}", prog, env!("CARGO_PKG_VERSION")); return; }
        "--list" => {
            for c in KNOWN { println!("{}", c); }
            return;
        }
        "--install" | "-s" => {
            let dir = argv.get(2).map(String::as_str).unwrap_or(".");
            install_symlinks(dir);
            return;
        }
        cmd if is_known(cmd) => {
            // Re-exec ourselves with argv[0] = cmd so each command's
            // internal `env::args().skip(1)` sees the right arguments.
            let err = std::process::Command::new(&argv[0])
                .arg0(cmd)
                .args(&argv[2..])
                .exec();
            eprintln!("{}: exec failed: {}", prog, err);
            std::process::exit(126);
        }
        cmd => {
            eprintln!("{}: unknown command: '{}'", prog, cmd);
            eprintln!("Try '{} --list' to see available commands.", prog);
            std::process::exit(127);
        }
    }
}

fn is_known(name: &str) -> bool {
    KNOWN.contains(&name)
}

// Each module's `main()` either calls `std::process::exit()` or returns
// normally (implicit exit 0). Both cases are handled here.
fn dispatch(name: &str) -> ! {
    match name {
        "clear"   => { clear::main(); }
        "command" => { command::main(); }
        "file"    => { file::main(); }
        "find"    => { find::main(); }
        "getconf" => { getconf::main(); }
        "grep"    => { grep::main(); }
        "sed"     => { sed::main(); }
        "wait"    => { wait::main(); }
        "which"   => { which::main(); }
        "ldd"     => { ldd::main(); }
        "dmesg"   => { dmesg::main(); }
        "ps"      => { ps::main(); }
        "pgrep"   => { pgrep::main(); }
        "pkill"   => { pkill::main(); }
        _ => unreachable!(),
    }
    // Reached only if the command returned without calling exit().
    std::process::exit(0);
}

fn install_symlinks(dir: &str) {
    let exe = match env::current_exe() {
        Ok(p) => p,
        Err(e) => { eprintln!("uutils-extra-multicall: current_exe: {}", e); std::process::exit(1); }
    };
    let base = Path::new(dir);
    if !base.is_dir() {
        eprintln!("uutils-extra-multicall: {}: not a directory", dir);
        std::process::exit(1);
    }
    for name in KNOWN {
        let target = base.join(name);
        if target.exists() || target.symlink_metadata().is_ok() {
            eprintln!("skip: {} already exists", target.display());
            continue;
        }
        match std::os::unix::fs::symlink(&exe, &target) {
            Ok(()) => println!("{} -> {}", target.display(), exe.display()),
            Err(e) => eprintln!("{}: {}", target.display(), e),
        }
    }
}

fn usage(prog: &str) {
    eprintln!("Usage: {} <command> [args...]", prog);
    eprintln!("       {} --list", prog);
    eprintln!("       {} --install [DIR]", prog);
    eprintln!();
    eprintln!("Or symlink {} to a command name (e.g. `ln -s {} grep`).", prog, prog);
    eprintln!();
    eprintln!("Commands:");
    for c in KNOWN { eprintln!("  {}", c); }
}
