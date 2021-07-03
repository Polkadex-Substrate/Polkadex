FROM ubuntu
RUN apt-get update
RUN apt-get install -y git && apt-get install -y curl
RUN git clone https://github.com/Polkadex-Substrate/Polkadex
RUN cd Polkadex && \
    git checkout develop && \
    ls && \
    apt-get install -y build-essential && \
    apt-get install -y clang && \
    apt-get install -y jq && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup toolchain install nightly-2021-05-11 && \
	rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-11 && \
	rustup default stable && \
	cargo update -p sp-std --precise e5437efefa82bd8eb567f1245f0a7443ac4e4fe7 && \
	cargo +nightly-2021-05-11 build --release "--$PROFILE"

EXPOSE 30333 9933 9944

CMD ["/Polkadex/target/release/polkadex-node"]