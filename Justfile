set dotenv-load

# Inspectify

inspectify-api ARGS="":
    RUST_LOG=debug cargo run -p inspectify-api -- {{ARGS}}

inspectify-app:
    cd inspectify-app && (npm install && npm run dev)

# CI/Release

release-patch args="":
    git checkout HEAD -- CHANGELOG.md
    cargo release patch {{args}}

build-ci:
    cargo build -p inspectify
    cargo build -p checko

release-hook:
    git cliff -t $NEW_VERSION -o CHANGELOG.md

# Debugging

checko-debug:
    # rm -rf example/runs.db3
    # rm -rf example/groups/
    RUST_LOG=debug cargo run -p inspectify-api -- --checko example

# Patch inspectify binaries

patch-inspectify-binaries-macos:
    cd inspectify-app && (npm install && npm run build)
    cargo zigbuild --target aarch64-apple-darwin     -p inspectify-api --release
    cargo zigbuild --target x86_64-apple-darwin      -p inspectify-api --release
    cargo zigbuild --target x86_64-pc-windows-gnu    -p inspectify-api --release
    cargo zigbuild --target x86_64-unknown-linux-gnu -p inspectify-api --release
    rm -rf inspectify-binaries
    git clone git@github.com:team-checkr/inspectify-binaries.git
    cp target/aarch64-apple-darwin/release/inspectify-api       inspectify-binaries/inspectify-macos-arm64
    cp target/x86_64-apple-darwin/release/inspectify-api        inspectify-binaries/inspectify-macos-x86_64
    cp target/x86_64-pc-windows-gnu/release/inspectify-api.exe  inspectify-binaries/inspectify-win.exe
    cp target/x86_64-unknown-linux-gnu/release/inspectify-api   inspectify-binaries/inspectify-linux
    strip inspectify-binaries/inspectify-macos-arm64
    strip inspectify-binaries/inspectify-macos-x86_64
    strip inspectify-binaries/inspectify-linux
    cd inspectify-binaries && git add . && git commit -m "Update binaries" && git push

WIN_REMOTE_HOST := "$WIN_REMOTE_HOST"
WIN_REMOTE_PATH := "$WIN_REMOTE_PATH"

patch-windows-machine:
    cd inspectify-app && npm run build
    cargo zigbuild --target x86_64-pc-windows-gnu -p inspectify-api --release
    ssh {{WIN_REMOTE_HOST}} taskkill /IM "inspectify-api.exe" /F
    scp target/x86_64-pc-windows-gnu/release/inspectify-api.exe {{WIN_REMOTE_HOST}}:{{WIN_REMOTE_PATH}}
    ssh {{WIN_REMOTE_HOST}} 'cmd.exe /c "cd {{WIN_REMOTE_PATH}} && inspectify-api.exe"'
