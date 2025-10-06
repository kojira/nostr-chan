FROM rust:1.86.0-bullseye

RUN apt-get update && \
    apt-get -y install git && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    rustup component add rust-src rustfmt clippy && \
    cargo install cargo-edit cargo-watch

# RUN apt-get install -y mecab libmecab-dev mecab-ipadic-utf8