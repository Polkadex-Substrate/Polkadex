cargo +nightly fmt
cargo +nightly build  --release
cargo +nightly clippy --release -- -D warnings
cargo +nightly test --release
