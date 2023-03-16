watch-wasm:
    cd inspectify/wasm; watchexec -w src -e rs "wasm-pack build --dev --target bundler"

watch-web:
    cd inspectify/ui; npm i && npm run dev

typeshare:
    #!/bin/bash
    set -e
    if which typeshare > /dev/null ; then
        typeshare . --lang=typescript --output-file=./inspectify/ui/src/lib/types.ts
    else
        echo "typeshare not run. to run install with 'cargo binstall typeshare-cli'"
    fi

build-wasm:
    cd inspectify/wasm; wasm-pack build --release --target bundler

build-ui: build-wasm typeshare
    cd inspectify/ui; npm i && npm run build

build-inspectify: build-ui
    cargo build -p inspectify --release

serve-inspectify:
    mkdir -p inspectify/ui/dist/
    # RUST_LOG=debug cargo run -p inspectify starters/fsharp-starter
    RUST_LOG=debug cargo run -p inspectify .

build-checko:
    cargo build -p checko --release

build-ci:
    cargo build -p inspectify
    cargo build -p checko

update-changelog:
    git cliff -o CHANGELOG.md

release-hook:
    git cliff -t $NEW_VERSION -o CHANGELOG.md

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
