FROM rust:1.86.0-bullseye

RUN apt-get update && \
    apt-get -y install git curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    rustup component add rust-src rustfmt clippy && \
    cargo install cargo-edit cargo-watch

# Install Node.js and npm for gemini-cli
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g @google/gemini-cli

# RUN apt-get install -y mecab libmecab-dev mecab-ipadic-utf8