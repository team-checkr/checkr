# Inspectify

inspectify-dev:
    RUST_BACKTRACE=full RUST_LOG=debug cargo watch -cx 'run -p inspectify'

inspectify-gen-api:
    abeye generate --target ts http://localhost:3000/spec -o inspectify-app/src/lib/api.ts

# CI/Release

release-patch args="":
    git checkout HEAD -- CHANGELOG.md
    cargo release patch {{args}}

build-ci:
    cargo build -p inspectify
    cargo build -p checko

release-hook:
    git cliff -t $NEW_VERSION -o CHANGELOG.md
