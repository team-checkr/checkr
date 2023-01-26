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

serve-inspectify:
    RUST_LOG=debug cargo run -p inspectify .

build-checko:
    cargo build -p checko --release

build-ci +target-flags:
    cross build -p inspectify --release {{target-flags}}
    cross build -p checko --release {{target-flags}}

# <registry URL>/<namespace>/<project>/<image>
IMAGE_NAME := "gitlab.gbar.dtu.dk/checkr-dev-env/demo-group-01/image:latest"

build-image:
    docker build . -f checko/Dockerfile -t {{IMAGE_NAME}}

push-image: build-image
    docker push {{IMAGE_NAME}}

full-competition: build-image
    cd checko; cargo run --release -- competition --base example --output competition.md example-config.toml

DEV_IMAGE_NAME := "checkr-dev"

build-dev-image:
    docker build . -f Dockerfile.dev -t {{DEV_IMAGE_NAME}}

docker-shell: build-dev-image
    docker run -it --rm -v $(realpath ./):/root/code {{DEV_IMAGE_NAME}} bash
