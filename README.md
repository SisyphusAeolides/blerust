# blerust

A blazing fast, robust, multi-line shell line editor written in Rust.
Designed as a modern, high-performance alternative to `ble.sh`.

## Features
- **Zero-latency** asynchronous-like syntax highlighting.
- **Transactional Undo/Redo**: Grouped actions (no more undoing single characters).
- **Fuzzy Tab Completion**: Intelligent command and path completion with `skim` fuzzy matching algorithm.
- **Fish-like Auto Suggestions**: Intelligent ghost text based on your command history.
- **Modal Editing**: Emacs and Vi (Normal/Insert) key bindings.
- **Multi-line Terminal Wrapping**: Gracefully handles long commands that wrap across terminal rows.

## Installation

### ArachOS repository

Install the ArachOS package from the configured repository:

```sh
sudo pacman -S blerust
```

Local packages are built and indexed by the ArachOS `build-packages` target.

### RPM-compatible systems

```sh
dnf copr enable sisyphuscode/blerust
dnf install blerust
```

After installing, run the setup command once to add blerust to your shell:

```sh
blerust --install
```

Then restart your terminal or source your shell config:

```sh
source ~/.bashrc
```

### From Source

```sh
cargo build --release
sudo install -m 755 target/release/blerust /usr/local/bin/blerust
blerust --install
source ~/.bashrc
```

## Shell Setup

The `blerust --install` command appends the following initialization snippet to
`~/.bashrc`:

```bash
# blerust initialization
if [[ $- == *i* && ${BLERUST_CHILD:-0} != 1 ]]; then exec blerust; fi
```

This replaces your interactive bash session with blerust on login, while
keeping subshells (commands run by blerust itself) unaffected via the
`BLERUST_CHILD` guard.

## Building
```sh
cargo build --release
```

## Running
```sh
cargo run --release
```
