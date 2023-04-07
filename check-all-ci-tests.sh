cargo fmt --check
RUSTFLAGS="-D warnings" cargo build
cargo build --features runtime-benchmarks
cargo clippy -- -D warnings
cargo test