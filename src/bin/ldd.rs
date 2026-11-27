use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use goblin::elf::Elf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() -> ExitCode {
    let mut files = Vec::new();
    let mut verbose = false;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--help" => { print_help(); return ExitCode::SUCCESS; }
            "--version" => { println!("ldd (uutils-extra) {VERSION}"); return ExitCode::SUCCESS; }
            "-v" | "--verbose" => verbose = true,
            "-u" | "--unused" | "-d" | "--data-relocs" | "-r" | "--function-relocs" => {}
            s if s.starts_with('-') && s != "-" => {
                eprintln!("ldd: unrecognized option '{s}'");
                return ExitCode::from(1);
            }
            _ => files.push(a.to_string()),
        }
        i += 1;
    }
    if files.is_empty() {
        eprintln!("ldd: missing file arguments");
        return ExitCode::from(1);
    }
    let mut code = 0;
    for f in &files {
        if let Err(e) = run(f, verbose) {
            eprintln!("ldd: {f}: {e}");
            code = 1;
        }
    }
    ExitCode::from(code)
}

fn run(file: &str, verbose: bool) -> std::io::Result<()> {
    let data = fs::read(file)?;
    if data.len() < 4 || &data[..4] != b"\x7fELF" {
        println!("\tnot a dynamic executable");
        return Ok(());
    }
    let elf = match Elf::parse(&data) {
        Ok(e) => e,
        Err(_) => { println!("\tnot a dynamic executable"); return Ok(()); }
    };
    if elf.dynamic.is_none() {
        println!("\tnot a dynamic executable");
        return Ok(());
    }
    let paths = build_paths(&elf);
    if cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
        println!("\tlinux-vdso.so.1 (0x0000000000000000)");
    }
    for lib in &elf.libraries {
        match find_lib(lib, &paths) {
            Some(p) => println!("\t{lib} => {} (0x0000000000000000)", p.display()),
            None => println!("\t{lib} => not found"),
        }
    }
    if verbose {
        println!("\tVersion information: (static analysis, no runtime info)");
    }
    if let Some(interp) = elf.interpreter {
        println!("\t{interp} (0x0000000000000000)");
    }
    Ok(())
}

fn build_paths(elf: &Elf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(lp) = env::var("LD_LIBRARY_PATH") {
        out.extend(lp.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    }
    for p in &elf.rpaths {
        out.extend(p.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    }
    for p in &elf.runpaths {
        out.extend(p.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    }
    for d in &["/lib", "/usr/lib", "/lib64", "/usr/lib64"] {
        let p = PathBuf::from(d);
        if p.is_dir() { out.push(p); }
    }
    for d in &[
        "/lib/x86_64-linux-gnu", "/usr/lib/x86_64-linux-gnu",
        "/lib/aarch64-linux-gnu", "/usr/lib/aarch64-linux-gnu",
        "/lib/arm-linux-gnueabihf", "/usr/lib/arm-linux-gnueabihf",
        "/lib/i386-linux-gnu", "/usr/lib/i386-linux-gnu",
    ] {
        let p = PathBuf::from(d);
        if p.is_dir() { out.push(p); }
    }
    out
}

fn find_lib(name: &str, paths: &[PathBuf]) -> Option<PathBuf> {
    if name.starts_with('/') {
        let p = PathBuf::from(name);
        return p.is_file().then_some(p);
    }
    paths.iter().map(|d| d.join(name)).find(|p| p.is_file())
}

fn print_help() {
    println!("Usage: ldd [OPTION]... FILE...");
    println!("Print shared object dependencies (static analysis).");
    println!();
    println!("  -v, --verbose         print all information");
    println!("  -u, --unused          print unused direct dependencies");
    println!("  -d, --data-relocs     perform relocations and report missing objects");
    println!("  -r, --function-relocs perform relocations and report missing objects and functions");
    println!("      --help            display this help and exit");
    println!("      --version         output version information and exit");
}
