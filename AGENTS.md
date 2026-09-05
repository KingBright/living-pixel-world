# Contributor / agent instructions

- All owned executable game, simulation, client and authoring logic is Rust. Do not add Python/JS/Lua gameplay, a web client or a second authoritative simulation.
- Read `docs/REQUIREMENTS.zh-CN.md` and `docs/SOURCE_STATUS.zh-CN.md` before changing scope. Original design aspirations are not all implemented. Preserve this distinction.
- Playable authority is `world_runtime::Game`. Commands, replay, native input and CLI must share `Action`. Presentation takes read-only state.
- Root Cargo workspace is std/local-only; the native application is deliberately a separate Cargo workspace. Run both build paths. Do not invent third-party lockfile checksums; resolve and commit a real native lockfile.
- Root entry commands: `cargo fmt --all`; `cargo test --workspace --locked --offline`; `cargo clippy --workspace --all-targets --locked --offline`. Native: `cargo build --manifest-path apps/world_client/Cargo.toml`, then optional `--features audio`.
- Initial candidate was not compiled/tested. Record real results, toolchain/OS and exact command. Never carry forward candidate text as evidence of a passing test.
- Preserve stable ids, deterministic ordering and fixed tick phase order. Wall clock and renderer randomness cannot enter authority. Test batching and reload equivalence when adding a cache.
- Resource transfer must debit its source and credit its destination exactly once. Contract and caravan escrow are owned funds. Craft ingredients are owned by a pending job. Keep checked capacity and no-reward-repeat behavior.
- New authoritative fields require codec/schema/version review, checksum coverage, corruption constraints and a test. Avoid silent incompatible save interpretation.
- No `todo!`, `unimplemented!` or decorative empty modules presented as complete features. Prefer a small real integrated implementation and state remaining product scope honestly.
- Existing RON examples are design artifacts; TSV is the active content format. Do not assume a RON compiler exists.
- Do not push main or change remote repositories without an explicit target and authorization. This artifact has not been pushed anywhere.
