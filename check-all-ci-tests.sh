cargo fmt
cargo build --release
cargo clippy --release -- -D warnings
cargo test --release
