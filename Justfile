# Inspectify

# Launch tmuxinator with the config for inspectify development
inspectify:
    tmuxinator

watch-web:
    cd inspectify/ui; npm i && npm run dev

watch-inspectify:
    mkdir -p inspectify/ui/dist/
    # RUST_LOG=debug cargo watch -i inspectify/ui/ -i starters/ -x 'run -p inspectify starters/fsharp-starter'
    RUST_LOG=debug cargo watch -i inspectify/ui/ -i starters/ -x 'run -p inspectify'

typeshare:
    #!/bin/bash
    set -e
    if which typeshare > /dev/null ; then
        typeshare . --lang=typescript --output-file=./inspectify/ui/src/lib/types.ts
    else
        echo "typeshare not run. to run install with 'cargo binstall typeshare-cli'"
    fi

build-ui: typeshare
    cd inspectify/ui; npm i && npm run build

build-inspectify: build-ui
    cargo build -p inspectify --release

# CI/Release

release-patch args="":
    git checkout HEAD -- CHANGELOG.md
    cargo release patch {{args}}

build-ci:
    cargo build -p inspectify
    cargo build -p checko

release-hook:
    git cliff -t $NEW_VERSION -o CHANGELOG.md
