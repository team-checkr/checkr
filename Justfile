set dotenv-load

# Inspectify

inspectify ARGS="":
    RUST_LOG=debug cargo run -p inspectify -- {{ARGS}}

inspectify-app:
    cd inspectify-app && (npm install && npm run dev)

# CI/Release

release-patch args="":
    git checkout HEAD -- CHANGELOG.md
    cargo release patch {{args}}

build-ui:
    cd inspectify-app && (npm install && npm run build)

release-hook:
    git cliff -t $NEW_VERSION -o CHANGELOG.md

# Debugging

checko-debug:
    # rm -rf example/runs.db3
    # rm -rf example/groups/
    RUST_LOG=debug cargo run -p inspectify -- --checko example

# Patch inspectify binaries

patch-inspectify-binaries-macos:
    cd inspectify-app && (npm install && npm run build)
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

WIN_REMOTE_HOST := "$WIN_REMOTE_HOST"
WIN_REMOTE_PATH := "$WIN_REMOTE_PATH"

patch-windows-machine:
    cd inspectify-app && npm run build
    cargo zigbuild --target x86_64-pc-windows-gnu -p inspectify --release
    ssh {{WIN_REMOTE_HOST}} taskkill /IM "inspectify.exe" /F
    scp target/x86_64-pc-windows-gnu/release/inspectify.exe {{WIN_REMOTE_HOST}}:{{WIN_REMOTE_PATH}}
    ssh {{WIN_REMOTE_HOST}} 'cmd.exe /c "cd /Users/oembo/checkr-test/fsharp-starter && inspectify.exe"'
