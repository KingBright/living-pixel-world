#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --locked --release --no-default-features --lib --bins --tests
cargo check --locked --all-targets --all-features
cargo build --locked --release --bins --example interaction_host --example interaction_review --example runtime_bench --example render_equivalence
cargo run --locked --release --no-default-features --example interaction_host -- exports/interaction-assets
cargo run --locked --release --no-default-features --example interaction_review -- exports/interactions-v010
cargo run --locked --release --no-default-features --example runtime_bench -- exports/runtime-benchmark-v010.json
