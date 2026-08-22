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

## Building
```sh
cargo build --release
```

## Running
```sh
cargo run --release
```
