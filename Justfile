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
