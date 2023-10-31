# Inspectify

inspectify-dev:
    RUST_BACKTRACE=full RUST_LOG=debug cargo watch --ignore .z3-trace -cx 'run -p inspectify'

# CI/Release

release-patch args="":
    git checkout HEAD -- CHANGELOG.md
    cargo release patch {{args}}

build-ci:
    cargo build -p inspectify
    cargo build -p checko

release-hook:
    git cliff -t $NEW_VERSION -o CHANGELOG.md
