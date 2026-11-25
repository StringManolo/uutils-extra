# uutils-extra

Additional basic command-line utilities missing from Rust's [uutils/coreutils](https://github.com/uutils/coreutils).

[![License: MIT](https://shields.io)](LICENSE)
[![Rust](https://shields.io)](https://rust-lang.org)

## Overview

`uutils-extra` aims to provide clean, fast, and cross-platform Rust implementations of essential Unix/Linux command-line tools that are either missing from `uutils/coreutils` or fall outside its core scope.

## Implemented Utilities

| Utility | Description | Status |
| :--- | :--- | :--- |
| `tool1` | Short description of what tool1 does | ✅ Ready |
| `tool2` | Short description of what tool2 does | 🚧 In progress |
| `tool3` | Short description of what tool3 does | 📋 Planned |

> placeholders

## Installation

### From Source

Ensure you have the [Rust toolchain](https://rustup.rs) installed, then clone and build the repository:

```bash
git clone https://github.com/stringmanolo/uutils-extra
cd uutils-extra
cargo build --release
```

The compiled binaries will be available in `target/release/`.

### Using Cargo

```bash
cargo install --git https://github.com
```

## Usage

Run any utility directly from your terminal:

```bash
./target/release/tool1 --help
```

## Contributing

Contributions are welcome! Whether it's implementing a new missing utility, improving POSIX compatibility, or fixing bugs:

1. Fork the repository.
2. Create a new branch (`git checkout -b feat/add-utility-name`).
3. Commit your changes (`git commit -m 'Add utility-name'`).
4. Push to the branch (`git push origin feat/add-utility-name`).
5. Open a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

