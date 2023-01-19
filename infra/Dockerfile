FROM rustlang/rust:nightly-slim

ENV CARGO_TARGET_DIR=/.cargo-target

RUN apt-get update && apt-get install -y pkg-config libssl-dev

WORKDIR /root/code
