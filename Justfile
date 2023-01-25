watch-wasm:
    cd inspectify/wasm; watchexec -w .. -e rs "wasm-pack build --dev --target bundler"

watch-web:
    cd inspectify/ui; npm run dev

typeshare:
    typeshare . --lang=typescript --output-file=./inspectify/ui/src/types.ts

build-wasm:
    cd inspectify/wasm; wasm-pack build --release --target bundler

build-ui: build-wasm typeshare
    cd inspectify/ui; npm i && npm run build

build-inspectify: build-ui
    cargo build -p inspectify --release

serve-inspectify: build-ui
    RUST_LOG=debug cargo run -p inspectify ./FsLexYacc-Starter

# x86_64-apple-darwin
# x86_64-pc-windows-msvc
# x86_64-unknown-linux-gnu
# aarch64-apple-darwin
# update-inspectify: build-inspectify
#     cp $(which inspectify) FsLexYacc-Starter/dev

# <registry URL>/<namespace>/<project>/<image>
IMAGE_NAME := "gitlab.gbar.dtu.dk/verification-lawyer-dev-env/demo-group-01/image:latest"

build-image:
    docker build . -t {{IMAGE_NAME}}

push-image: build-image
    docker push {{IMAGE_NAME}}

docker-shell: build-image
    docker run -it --rm -v $(realpath ./):/root/code vl-infra bash

full-competition: build-image
    cd infra; cargo run --release -- competition --base example --output competition.md example-config.toml
