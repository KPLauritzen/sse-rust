test:
    cargo test

bench:
    cargo bench

build:
    cargo build --release

wasm:
    wasm-pack build --target web

deploy-wasm: wasm
    cp pkg/sse_core.js pkg/sse_core_bg.wasm ../kplauritzen.github.io/docs/wasm/
