test:
    cargo test

bench:
    cargo bench

research:
    cargo run --profile dist --features research-tools --bin research_harness -- --cases research/cases.json --format pretty

research-json:
    cargo run --profile dist --features research-tools --bin research_harness -- --cases research/cases.json --format json

research-json-save stamp:
    mkdir -p research/runs
    cargo run --profile dist --features research-tools --bin research_harness -- --cases research/cases.json --format json > research/runs/{{stamp}}.json

build:
    cargo build --release

build-dist:
    cargo build --profile dist

build-tools:
    cargo build --release --features research-tools --bins

wasm:
    wasm-pack build --target web -- --features wasm-bindings

deploy-wasm: wasm
    cp pkg/sse_core.js pkg/sse_core_bg.wasm ../kplauritzen.github.io/docs/wasm/
