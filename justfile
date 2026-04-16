test:
    cargo test

check-k3-graph-merge:
    cargo build --release --bin search
    timeout 15s target/release/search 1,3,2,1 1,6,1,1 --max-lag 22 --max-intermediate-dim 5 --max-entry 6 --search-mode graph-only --json | grep -q '"outcome": "equivalent"'

bench:
    cargo bench

bench-search *criterion_args:
    cargo bench --bench search -- {{criterion_args}}

bench-search-save-baseline name *criterion_args:
    cargo bench --bench search -- --save-baseline {{name}} {{criterion_args}}

bench-search-compare-baseline name *criterion_args:
    cargo bench --bench search -- --baseline {{name}} {{criterion_args}}

research:
    cargo run --profile dist --features research-tools --bin research_harness -- --cases research/cases.json --format pretty

research-json:
    cargo run --profile dist --features research-tools --bin research_harness -- --cases research/cases.json --format json

research-json-save stamp:
    mkdir -p research/runs
    cargo run --profile dist --features research-tools --bin research_harness -- --cases research/cases.json --format json > research/runs/{{stamp}}.json

graph-proposal-shortlist-round *args:
    cargo build --profile dist --features research-tools --bin compare_graph_move_proposals
    timeout -k 5s 20s target/dist/compare_graph_move_proposals \
        --fixture-ref research/fixtures/brix_ruiz_family.json#brix_ruiz_k3 \
        --seeded-guide-id endpoint_16_path \
        --current guide:1 \
        --target guide:15 \
        --max-dim 4 \
        --zigzag-bridge-entry 8 \
        --top-k 6 \
        --probe-lag 3 \
        --probe-shortlist-k 4 \
        {{args}}

build:
    cargo build --release

build-dist:
    cargo build --profile dist

build-tools:
    cargo build --release --features research-tools --bins
