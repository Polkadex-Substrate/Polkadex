FROM bitnami/git:latest AS builder

RUN apt-get update && apt-get install --assume-yes curl build-essential cmake clang jq

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
  export PATH="$PATH:$HOME/.cargo/bin" && \
  rustup toolchain install nightly && \
  rustup target add wasm32-unknown-unknown --toolchain nightly && \
  rustup default nightly && \
  git clone https://github.com/Polkadex-Substrate/Polkadex -b mainnet-release && \
  cd Polkadex && \
  git checkout $(git describe --tags --abbrev=0) && \
  cargo build --release

# /\-Build Stage | Final Stage-\/

FROM docker.io/library/ubuntu:20.04
COPY --from=builder /Polkadex/target/release/polkadex-node /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /polkadex-node polkadex-node && \
        mkdir -p /polkadex-node/.local/share && \
        mkdir /data && \
        chown -R polkadex-node:polkadex-node /data && \
        ln -s /data /polkadex-node/.local/share/polkadex-node && \
        rm -rf /usr/bin /usr/sbin

COPY --from=builder /Polkadex/extras/customSpecRaw.json /data

USER polkadex-node
EXPOSE 30333 9933 9944
VOLUME ["/data"]

EXPOSE 30333 9933 9944

ENTRYPOINT ["/usr/local/bin/polkadex-node"]

# You should be able to run a validator using this docker image in a bash environmment with the following command:
# docker run <docker_image_name> --chain /data/customSpecRaw.json $(curl -s https://raw.githubusercontent.com/Polkadex-Substrate/Polkadex/main/docs/run-a-validator.md | grep -o -m 1 -E "\-\-bootnodes \S*") --validator --name "Validator-Name"
