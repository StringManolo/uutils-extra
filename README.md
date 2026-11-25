# uutils-extra

Additional basic command-line utilities missing from Rust's [uutils/coreutils](https://github.com/uutils/coreutils).

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

## Overview

`uutils-extra` aims to provide clean, fast, and cross-platform Rust implementations of useful command-line utilities that are either missing from `uutils/coreutils` or fall outside its core scope.

The project focuses on portability, low resource usage, straightforward command-line interfaces, and compatibility with Unix-like systems.

## Implemented Utilities

| Utility | Description | Status |
| :--- | :--- | :--- |
| `which` | Locate executable files in the user's `PATH`. | 🚧 In progress |

## Installation

### Download a precompiled binary

Precompiled binaries are published in [GitHub Releases](https://github.com/StringManolo/uutils-extra/releases).

Each release may include builds for multiple architectures and environments, including:

- Linux GNU (glibc)
- Linux musl
- Android (Bionic)
- x86 and x86_64
- ARMv6, ARMv7 and AArch64
- RISC-V and other supported architectures

Download the archive matching your system, extract it, and place the executable somewhere in your `PATH`.

For example:

```bash
tar -xzf uutils-extra-*.tar.gz
chmod +x which
./which sh
```

Check `SHA256SUMS` to verify downloaded archives.

### Build from source

Make sure you have the [Rust toolchain](https://rustup.rs/) installed.

```bash
git clone https://github.com/StringManolo/uutils-extra.git
cd uutils-extra

cargo build --release --bins
```

The compiled executables will be available in:

```text
target/release/
```

### Install using Cargo

```bash
cargo install --git https://github.com/StringManolo/uutils-extra --bin which
```

## Usage

```bash
which sh
which bash
which -a python3
which --help
```

## Development

Format the code:

```bash
cargo fmt --all
```

Run all tests:

```bash
cargo test --all-targets
```

Build all binaries:

```bash
cargo build --release --bins
```

## Automated Builds and Releases

GitHub Actions is used to cross-compile binaries for supported targets and publish them as GitHub Releases.

To create a release, push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow is triggered automatically by tags matching `v*`.

All architecture builds are published in a single GitHub Release for that version, together with SHA-256 checksums.

## Contributing

Contributions are welcome! Whether it's implementing a new utility, improving POSIX compatibility, adding platform support, or fixing bugs:

1. Fork the repository.
2. Create a branch (`git checkout -b feat/add-utility-name`).
3. Commit your changes (`git commit -m "Add utility-name"`).
4. Push your branch (`git push origin feat/add-utility-name`).
5. Open a Pull Request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
