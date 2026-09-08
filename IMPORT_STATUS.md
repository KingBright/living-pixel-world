# 0.8 upload status

Date: 2026-09-08.
Repository: `KingBright/living-pixel-world`.
Requested destination: `main` only, with no new branch or PR.

## Actual result

The GitHub connector successfully accepted source writes, including Rust code, content data and CI configuration. The current main ref was read again before the status commit. Full source publication is **incomplete**.

- Complete artifact: 184 files, 1,198,614 bytes.
- Original local source commit: `2812ae87e1c01fe4aed64dab936e0b09a784b43f`.
- Original full source tree: `c0edfb50d7f6335812ca61157bdad6158349ac6f`.
- Successfully uploaded original files: 61.
- Last accepted partial tree: `abb66789c7e1050af555b20127eba077c2a9e0f8`.
- Original files still absent from that tree: 123.
- Blocked path: `crates/world_runtime/src/ecology.rs`.
- Platform response: `因 OpenAI 无法确定请求的安全状态，已拦截此工具调用。`
- One identical retry was made and received the same denial. The denied file was not sent via another route.
- The partial tree is not attached to `main`. Main contains publication documentation, not the complete game.
- No new branch or PR was created. Existing branches and the older draft PR were not deleted or merged.
- No build, test, native launch or gameplay validation occurred during publication.

The original ZIP and Git bundle were verified locally. Each accepted cumulative tree matched its independently computed expected Git tree hash. Those checks prove the uploaded bytes for the accepted portion, not full publication or game correctness.

## Pending original files

```text
crates/world_runtime/src/ecology.rs
crates/world_runtime/src/economy.rs
crates/world_runtime/src/experience_protocol.rs
crates/world_runtime/src/field_guide.rs
crates/world_runtime/src/game.rs
crates/world_runtime/src/geology.rs
crates/world_runtime/src/interaction_protocol.rs
crates/world_runtime/src/interactions.rs
crates/world_runtime/src/locomotion.rs
crates/world_runtime/src/navigation.rs
crates/world_runtime/src/objectives.rs
crates/world_runtime/src/protocol.rs
crates/world_runtime/src/quests.rs
crates/world_runtime/src/resources.rs
crates/world_runtime/src/scenarios.rs
crates/world_runtime/src/session.rs
crates/world_runtime/src/society.rs
crates/world_runtime/src/storage.rs
crates/world_runtime/src/streaming.rs
crates/world_runtime/src/targets.rs
crates/world_runtime/src/terrain.rs
crates/world_runtime/src/types.rs
crates/world_runtime/src/waterworks.rs
crates/world_runtime/src/weather.rs
crates/world_runtime/src/work.rs
crates/world_runtime/src/work_protocol.rs
crates/world_runtime/src/work_runtime.rs
crates/world_runtime/tests/acceptance.rs
crates/world_runtime/tests/autonomy_production.rs
crates/world_runtime/tests/experience.rs
crates/world_runtime/tests/interaction_outcomes.rs
crates/world_runtime/tests/living_systems.rs
crates/world_runtime/tests/migration_v6.rs
crates/world_runtime/tests/work_acceptance.rs
crates/world_view/Cargo.toml
crates/world_view/src/actors.rs
crates/world_view/src/atelier_ui.rs
crates/world_view/src/audio.rs
crates/world_view/src/experience_ui.rs
crates/world_view/src/lib.rs
crates/world_view/src/text.rs
crates/world_view/src/work_ui.rs
crates/world_view/tests/atelier_ui.rs
crates/world_view/tests/experience_ui.rs
crates/world_view/tests/interaction_ui.rs
crates/world_view/tests/read_only.rs
crates/world_view/tests/work_ui.rs
docs/ACCEPTANCE_V07.zh-CN.md
docs/ACCEPTANCE_V08.zh-CN.md
docs/ARCHITECTURE.md
docs/CONTENT_RUNTIME.md
docs/CONTENT_SCHEMA.md
docs/DESIGN.md
docs/IMPLEMENTATION_PLAN.zh-CN.md
docs/INTERACTION_OUTCOMES.zh-CN.md
docs/LIVING_SYSTEMS.zh-CN.md
docs/PROTOCOL.md
docs/PROVENANCE.md
docs/REQUIREMENTS.zh-CN.md
docs/REUSABLE_WORK.zh-CN.md
docs/SAVE_FORMAT.md
docs/SOURCE_STATUS.zh-CN.md
docs/V07_PLAYABLE_LOOP.zh-CN.md
docs/V08_AUTONOMY_PRODUCTION.zh-CN.md
docs/VALIDATION.md
docs/archive/m1a/AGENTS.md
docs/archive/m1a/IMPLEMENTATION_PLAN.zh-CN.md
docs/archive/m1a/PROVENANCE.md
docs/archive/m1a/README.md
docs/archive/m1a/VALIDATION.md
docs/archive/v0.3/SOURCE_STATUS.zh-CN.md
docs/archive/v0.3/VALIDATION.md
docs/archive/v0.4/IMPLEMENTATION_PLAN.zh-CN.md
docs/archive/v0.4/README.md
docs/archive/v0.4/SAVE_FORMAT.md
docs/archive/v0.4/SOURCE_STATUS.zh-CN.md
docs/archive/v0.4/VALIDATION.md
docs/archive/v0.5/AGENTS.md
docs/archive/v0.5/IMPLEMENTATION_PLAN.zh-CN.md
docs/archive/v0.5/README.md
docs/archive/v0.5/SAVE_FORMAT.md
docs/archive/v0.5/SOURCE_STATUS.zh-CN.md
docs/archive/v0.5/VALIDATION.md
docs/archive/v0.6/AGENTS.md
docs/archive/v0.6/IMPLEMENTATION_PLAN.zh-CN.md
docs/archive/v0.6/README.md
docs/archive/v0.6/SOURCE_STATUS.zh-CN.md
docs/archive/v0.6/VALIDATION.md
docs/archive/v0.7/AGENTS.md
docs/archive/v0.7/README.md
docs/archive/v0.7/SOURCE_STATUS.zh-CN.md
docs/archive/v0.7/VALIDATION.md
examples/bridge-outcomes.lpw
examples/bridge-work.lpw
examples/experience-queries.lpw
examples/experience-wait.lpw
examples/hydrology.lpw
examples/irrigation-outcomes.lpw
examples/irrigation-work.lpw
examples/living-systems.lpw
examples/market.lpw
examples/smoke.lpw
examples/work-refund.lpw
examples/workshop-cancel.lpw
examples/workshop-npc.lpw
examples/workshop-player.lpw
tools/validate.rs
verification/archive/m1a/changes_from_m0.json
verification/archive/m1a/reference_checks.json
verification/archive/m1a/static_checks.json
verification/archive/v0.3/MANIFEST.sha256
verification/changes_from_m1a.json
verification/changes_from_v0_4.json
verification/changes_from_v0_6.json
verification/execution_attempts_v0_6.json
verification/execution_attempts_v0_7.json
verification/execution_attempts_v0_8.json
verification/source_checks_v0_3.json
verification/source_checks_v0_4.json
verification/source_checks_v0_5.json
verification/source_checks_v0_6.json
verification/source_checks_v0_7.json
verification/source_checks_v0_8.json
```

## Completion condition

Publish the complete source through operations permitted by the platform. Verify all original files against the full source tree above before calling the transfer complete. Any separately authorized documentation changes must be tracked as such rather than misreported as byte-identical source. Verify the final main ref after its non-force update. Keep the owner-requested direct-main workflow.
