# 0.3.0 source candidate

Expanded the retained M1a water/terrain base into an integrated Rust runtime, read-only pixel renderer, native host and agent-oriented CLI. Added actual weather/ecology/economy/NPC/contract/player state, actions, snapshots and replay; 80 items, 54 recipes, 24 plants, 13 animal/cohort definitions and 10 authored anchors. The root has nine local-only packages, with the native host kept in a separate Cargo workspace.

Added integration tests, three executable scenario sources, documentation and CI configuration. Source checks counted 66 test declarations and 32 Rust source files. No compiler, Rust test, native launch, cross-platform or long-run result is asserted. Unimplemented final requirements are explicit in `docs/SOURCE_STATUS.zh-CN.md`.

Original design/requirements were retained unchanged. Historical M1a documents and reference checks were archived. No GitHub push or Library mutation occurred.
