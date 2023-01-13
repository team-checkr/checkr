watch-wasm:
    cd wasm; watchexec -w .. -e rs "wasm-pack build --release --target web"

watch-web:
    cd ui; npm run dev

build-wasm:
    cd wasm; wasm-pack build --release --target web

build-ui: build-wasm
    cd ui; npm i && npm run build

serve-api:
    cargo run -p api
