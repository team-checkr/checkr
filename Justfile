watch-wasm:
    cd wasm; watchexec -w .. -e rs "wasm-pack build --release --target bundler"

watch-web:
    cd ui; npm run dev

build-wasm:
    cd wasm; wasm-pack build --release --target bundler

build-ui: build-wasm
    cd ui; npm i && npm run build

build-api: build-ui
    cargo build -p api --release

serve-api:
    cargo run -p api
