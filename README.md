# iopole

Command-line tool for the Iopole platform.

## Usage

```sh
cargo run -- --help
cargo run -- login
cargo run -- logout
cargo run -- sync
```

## Installation

```sh
cargo install --path .
```

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
