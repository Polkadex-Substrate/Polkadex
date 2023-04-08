cargo fmt --check
cargo clippy -- -D warnings
RUSTFLAGS="-D warnings" cargo build
cargo test
cargo build --features runtime-benchmarks