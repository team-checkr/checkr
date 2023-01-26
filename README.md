# Checkr

![](inspectify-screenshot.png)

## Architecture

The checkr toolchain is split up into multiple crates:

- `checkr`: Contains the fundamental types and functions for the core analysis analysis and validation of results.
- `checko`: Contains the infrastructure code for running external implementations for the analysis.
- `inspectify`: Contains the application code for displaying analysis external implementations.

Each of the crates have different target audiences: `checko` is meant for admin tasks, such as correcting assignments, running competitions, and for validating submissions in CI. `inspectify` is meant for students to interact with their analysis tool in a user-friendly way. `checkr` is the core analysis implementation, and is purely meant to be used as a dependency in other crates.

To learn more about [checko](./checko/README.md) and [inspectify](./inspectify/README.md), checkout the README in their folders.

## Requirements

- Rust and Cargo
- Just (`cargo install just`)
- Typeshare (`cargo install typeshare-cli`)
- wasm-pack ([https://rustwasm.github.io/wasm-pack/installer/])
- Node and npm

## Windows setup steps

- Install Git using winget
- Install Rust using `rustup-init.exe` from https://rustup.rs
- Install binstall with `cargo install cargo-binstall`
- Install just with `cargo binstall just`
- Install typeshare with `cargo binstall typeshare`
- Install Node with `winget install OpenJS.NodeJS`
- Install wasm-pack with `wasm-pack-init.exe` from https://rustwasm.github.io/wasm-pack/installer/
