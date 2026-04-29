set dotenv-load

app app:
    just build-wasm
    cd apps/{{app}} && (npm install && npm run dev)

app-build app:
    just build-wasm
    cd apps/{{app}} && (npm install && npm run build)

build-wasm:
    cd crates/chip-wasm && wasm-pack build --target web --release

# Inspectify

inspectify ARGS="":
    RUST_LOG=debug cargo run -p inspectify -- {{ARGS}}
inspectify-test ARGS="":
    RUST_LOG=debug cargo run -p inspectify -- {{ARGS}} --driver true ./starters/fsharp-starter
#./starters/Group-24-Pizza-Pigeons/code
inspectify-app:
    cd apps/inspectify && (npm install && npm run dev)

# CI/Release

release-patch args="":
    git checkout HEAD -- CHANGELOG.md
    cargo release --exclude mcltl patch {{args}}

release-minor args="":
    git checkout HEAD -- CHANGELOG.md
    cargo release --exclude mcltl minor {{args}}

build-ui:
    cd apps/inspectify && (npm install && npm run build)

release-hook:
    git cliff -t $NEW_VERSION -o CHANGELOG.md

# Debugging

checko-debug:
    # rm -rf example/runs.db3
    # rm -rf example/groups/
    # rm -rf ../2024/checko-data/runs.db3
    # rm -rf ../2024/checko-data/groups/
    RUST_LOG=debug cargo run --release -p inspectify -- --checko ../checko-data
    # CARGO_PROFILE_RELEASE_DEBUG=true RUST_LOG=debug cargo flamegraph --root -p inspectify -- --checko ../2024/checko-data

# Patch inspectify binaries

patch-inspectify-binaries-macos:
    cd apps/inspectify && (npm install && npm run build)
    cargo zigbuild --target aarch64-apple-darwin     -p inspectify --release
    cargo zigbuild --target x86_64-apple-darwin      -p inspectify --release
    cargo zigbuild --target x86_64-pc-windows-gnu    -p inspectify --release
    cargo zigbuild --target x86_64-unknown-linux-gnu -p inspectify --release
    rm -rf inspectify-binaries
    git clone git@github.com:team-checkr/inspectify-binaries.git
    cp target/aarch64-apple-darwin/release/inspectify       inspectify-binaries/inspectify-macos-arm64
    cp target/x86_64-apple-darwin/release/inspectify        inspectify-binaries/inspectify-macos-x86_64
    cp target/x86_64-pc-windows-gnu/release/inspectify.exe  inspectify-binaries/inspectify-win.exe
    cp target/x86_64-unknown-linux-gnu/release/inspectify   inspectify-binaries/inspectify-linux
    strip inspectify-binaries/inspectify-macos-arm64
    strip inspectify-binaries/inspectify-macos-x86_64
    strip inspectify-binaries/inspectify-linux
    cd inspectify-binaries && git add . && git commit -m "Update binaries" && git push

CHECKO_REMOTE_HOST := "$CHECKO_REMOTE_HOST"
CHECKO_REMOTE_PATH := "$CHECKO_REMOTE_PATH"

patch-checko:
    #!/bin/bash
    export PUBLIC_API_BASE=""
    export PUBLIC_CHECKO="yes"
    (cd apps/inspectify && npm run build)
    cargo zigbuild --target x86_64-unknown-linux-gnu -p inspectify --release
    scp target/x86_64-unknown-linux-gnu/release/inspectify {{CHECKO_REMOTE_HOST}}:{{CHECKO_REMOTE_PATH}}

WIN_REMOTE_HOST := "$WIN_REMOTE_HOST"
WIN_REMOTE_PATH := "$WIN_REMOTE_PATH"

patch-windows-machine:
    cd apps/inspectify && npm run build
    cargo zigbuild --target x86_64-pc-windows-gnu -p inspectify --release
    ssh {{WIN_REMOTE_HOST}} taskkill /IM "inspectify.exe" /F
    scp target/x86_64-pc-windows-gnu/release/inspectify.exe {{WIN_REMOTE_HOST}}:{{WIN_REMOTE_PATH}}
    ssh {{WIN_REMOTE_HOST}} 'cmd.exe /c "cd /Users/oembo/checkr-test/fsharp-starter && inspectify.exe"'

# Download the latest binaries from https://github.com/team-checkr/checkr/releases/latest
update-inspectify-binaries:
    #!/bin/bash
    git clone --depth 1 git@github.com:team-checkr/inspectify-binaries.git || true
    cd inspectify-binaries
    git pull

    rm -rf temp
    mkdir temp
    cd temp

    curl -fL -o "inspectify-aarch64-apple-darwin.tar.xz"  "https://github.com/team-checkr/checkr/releases/latest/download/inspectify-aarch64-apple-darwin.tar.xz"
    curl -fL -o "inspectify-x86_64-apple-darwin.tar.xz" "https://github.com/team-checkr/checkr/releases/latest/download/inspectify-x86_64-apple-darwin.tar.xz"
    curl -fL -o "inspectify-x86_64-pc-windows-msvc.zip"      "https://github.com/team-checkr/checkr/releases/latest/download/inspectify-x86_64-pc-windows-msvc.zip"
    curl -fL -o "inspectify-x86_64-unknown-linux-gnu.tar.xz" "https://github.com/team-checkr/checkr/releases/latest/download/inspectify-x86_64-unknown-linux-gnu.tar.xz"

    # Extract each archive into the same temp dir
    tar -xJf "inspectify-aarch64-apple-darwin.tar.xz"
    tar -xJf "inspectify-x86_64-apple-darwin.tar.xz"
    unzip -o "inspectify-x86_64-pc-windows-msvc.zip" -d "inspectify-x86_64-pc-windows-msvc" >/dev/null 2>&1
    tar -xJf "inspectify-x86_64-unknown-linux-gnu.tar.xz"

    cp "inspectify-aarch64-apple-darwin/inspectify" ../inspectify-macos-arm64
    cp "inspectify-x86_64-apple-darwin/inspectify" ../inspectify-macos-x86_64
    cp "inspectify-x86_64-pc-windows-msvc/inspectify.exe" ../inspectify-win.exe
    cp "inspectify-x86_64-unknown-linux-gnu/inspectify" ../inspectify-linux

    cd ..
    rm -rf temp

    strip inspectify-macos-arm64 || true
    strip inspectify-macos-x86_64 || true
    strip inspectify-linux || true

    if [ -n "$(git status --porcelain)" ]; then
        git add . && git commit -m "Update binaries from checkr release" && git push
    else
        echo "No changes to commit"
    fi
