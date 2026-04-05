test:
    cargo test

bench:
    cargo bench

research:
    cargo run --release --bin research_harness -- --cases research/cases.json --format pretty

research-json:
    cargo run --release --bin research_harness -- --cases research/cases.json --format json

research-json-save stamp:
    mkdir -p research/runs
    cargo run --release --bin research_harness -- --cases research/cases.json --format json > research/runs/{{stamp}}.json

build:
    cargo build --release

wasm:
    wasm-pack build --target web

deploy-wasm: wasm
    cp pkg/sse_core.js pkg/sse_core_bg.wasm ../kplauritzen.github.io/docs/wasm/
