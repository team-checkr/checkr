FROM rust:1.68.1-slim as checko

ENV CARGO_TARGET_DIR=/.cargo-target

WORKDIR /root/

RUN apt-get update && apt-get install -y pkg-config libssl-dev

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN cargo help

COPY . /root/code

WORKDIR /root/code

RUN \
    --mount=type=cache,target=/.cargo-target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --release -p checko && \
    cp /.cargo-target/release/checko /root/checko

FROM mcr.microsoft.com/dotnet/sdk:7.0-bullseye-slim

COPY --from=checko /root/checko /usr/bin
