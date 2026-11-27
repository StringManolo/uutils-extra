# uutils-extra

Additional basic command-line utilities missing from Rust's [uutils/coreutils](https://github.com/uutils/coreutils).

> **Early development:** Utilities are not yet mature and compatibility may vary.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

## Overview

`uutils-extra` provides lightweight command-line utilities implemented in Rust, complementing [uutils/coreutils](https://github.com/uutils/coreutils).

It focuses on portability, low resource usage and support for a wide range of Linux architectures, including GNU, musl and Android Bionic environments.

## Available Utilities

`clear`, `command`, `dmesg`, `file`, `find`, `getconf`, `grep`, `ldd`, `pgrep`, `pkill`, `ps`, `sed`, `wait` and `which`.

## Installation

Download the archive matching your system from [GitHub Releases](https://github.com/StringManolo/uutils-extra/releases), extract it and place the executables somewhere in your `PATH`.

Release archives include:

- x86, ARM, AArch64, MIPS, PowerPC, RISC-V, LoongArch64, s390x and SPARC64.
- GNU and musl Linux targets, where supported.
- Android Bionic targets.

Verify downloads using the included `SHA256SUMS` file:

```bash
sha256sum -c SHA256SUMS
```

## Multicall vs. Individual Binaries

There are two ways to use `uutils-extra`.

### Multicall (recommended for multiple utilities)

The `uutils-extra-multicall` binary includes **all available utilities in a single executable**.

It is lighter than installing all individual binaries separately because the shared executable contains the implementations without duplicating the executable overhead for every command.

Use multicall if you want most or all of the utilities.

List available commands:

```bash
uutils-extra-multicall --list
```

Run a command directly:

```bash
uutils-extra-multicall grep "pattern" file.txt
uutils-extra-multicall find . -name "*.rs"
uutils-extra-multicall ps
```

#### Create command symlinks

Multicall can also behave like individual commands through symbolic links.

For example:

```bash
ln -s /usr/bin/uutils-extra-multicall /usr/bin/grep
ln -s /usr/bin/uutils-extra-multicall /usr/bin/find
ln -s /usr/bin/uutils-extra-multicall /usr/bin/ps
```

When invoked through a symlink, multicall automatically detects the command name and executes the corresponding utility.

Alternatively, install symlinks for every available command:

```bash
uutils-extra-multicall --install
```

By default, symlinks are created in the current directory. You can specify another directory:

```bash
uutils-extra-multicall --install /usr/local/bin
```

**Note:** The destination directory must exist and you need sufficient permissions to create the symlinks.

### Individual binaries

If you only need one or two utilities, use their individual executables instead of multicall.

For example:

```bash
./which bash
./grep "pattern" file.txt
```

Individual binaries avoid including utilities you don't need.

**In short:**
- A few commands: use individual binaries.
- Most or all commands: use `uutils-extra-multicall`.

## Usage

Each utility supports its own command-line interface. Use `--help` to see available options.

```bash
which --help
grep --help
find --help
```

## Development

```bash
cargo build --release --bins
cargo test --all-targets
```

## Automated Builds and Releases

GitHub Actions automatically cross-compiles all binaries found in `src/bin/` for the configured targets whenever a version tag matching `v*` is pushed.

Each release includes architecture-specific `.tar.gz` archives containing the executables, `README.md` and `LICENSE`, along with SHA-256 checksums.

Create a release with:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Contributing

Contributions are welcome! Feel free to submit bug fixes, improvements, new utilities and platform support through Pull Requests.

## License

MIT License. See [LICENSE](LICENSE).
