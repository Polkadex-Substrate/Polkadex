FROM ubuntu:22.04

COPY ./polkadex-node /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /polkadex-node polkadex-node && \
    mkdir -p /polkadex-node/.local/share && \
    mkdir /data && \
    chown -R polkadex-node:polkadex-node /data && \
    ln -s /data /polkadex-node/.local/share/polkadex-node && \
    rm -rf /usr/bin /usr/sbin

COPY ./extras/customSpecRaw.json /data

USER polkadex-node

EXPOSE 30333 9933 9944
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/polkadex-node"]
