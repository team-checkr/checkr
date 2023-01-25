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

# build-inspectify-all: build-ui
build-inspectify-all:
    # cross build -p inspectify --release --target aarch64-apple-darwin
    # cross build -p inspectify --release --target x86_64-apple-darwin
    cross build -p inspectify --release --target x86_64-unknown-linux-gnu
    cargo xwin build -p inspectify --release

serve-inspectify: build-ui
    RUST_LOG=debug cargo run -p inspectify ./FsLexYacc-Starter

# x86_64-apple-darwin
# x86_64-pc-windows-msvc
# x86_64-unknown-linux-gnu
# aarch64-apple-darwin
# update-inspectify: build-inspectify
#     cp $(which inspectify) FsLexYacc-Starter/dev

build-checko:
    cargo build -p checko --release

# <registry URL>/<namespace>/<project>/<image>
IMAGE_NAME := "gitlab.gbar.dtu.dk/checkr-dev-env/demo-group-01/image:latest"

build-image:
    docker build . -t {{IMAGE_NAME}}

push-image: build-image
    docker push {{IMAGE_NAME}}

docker-shell: build-image
    docker run -it --rm -v $(realpath ./):/root/code {{IMAGE_NAME}} bash

full-competition: build-image
    cd checko; cargo run --release -- competition --base example --output competition.md example-config.toml

DEV_IMAGE_NAME := "checkr-dev"

build-dev-image:
    docker build . -f Dockerfile.dev -t {{DEV_IMAGE_NAME}}

docker-dev: build-dev-image
    docker run -it --rm -v /var/run/docker.sock:/var/run/docker.sock -v $(realpath ./):/root/code {{DEV_IMAGE_NAME}} bash
