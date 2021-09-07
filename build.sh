sudo apt install -y git clang curl libssl-dev llvm libudev-dev
curl https://getsubstrate.io -sSf | bash -s -- --fast
rustup default stable
rustup update
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
cargo build --release