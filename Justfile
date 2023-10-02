# CI/Release

release-patch args="":
    git checkout HEAD -- CHANGELOG.md
    cargo release patch {{args}}

build-ci:
    cargo build -p ui
    cargo build -p checko

release-hook:
    git cliff -t $NEW_VERSION -o CHANGELOG.md
