# Partial source import: not buildable yet

This branch is an incomplete transfer of the previously delivered Living Pixel World 0.3 source candidate. Do not treat this PR as a complete project or merge it as a finished import.

## Verified state

- The repository is `KingBright/living-pixel-world`, currently public.
- The source artifact contains 86 files. Its full Git tree is `835d14c7c685234f1b24528c6a745b6664b80af5`.
- 25 original artifact files, including `.gitignore` already on `main`, are present in this branch. Their bytes match the delivered source artifact.
- 61 original files have NOT been uploaded. The gameplay runtime, native client, renderer, content data, CI and documentation are incomplete or absent here.
- The platform safety check could not determine the safety status of the next source-upload request. The identical request was retried once and was blocked again. No alternate route was used to submit the blocked content.
- No Rust compilation, tests, native launch or cross-platform validation were performed.
- This status file is additional transfer documentation, not part of the 86-file source artifact. The original README describes the full candidate, not the partial contents of this branch.

## Original files still missing

```text
.github/workflows/ci.yml
MANIFEST.sha256
apps/world_client/Cargo.toml
apps/world_client/src/main.rs
assets/catalog/anchors.tsv
assets/catalog/animals.tsv
assets/catalog/items.tsv
assets/catalog/plants.tsv
assets/catalog/recipes.tsv
assets/content/items/iron_axe.ron
assets/content/recipes/iron_axe.ron
assets/content/settlements/river_market_town.ron
assets/content/species/temperate_oak.ron
crates/world_cli/Cargo.toml
crates/world_cli/src/main.rs
crates/world_runtime/Cargo.toml
crates/world_runtime/src/actions.rs
crates/world_runtime/src/codec.rs
crates/world_runtime/src/content.rs
crates/world_runtime/src/ecology.rs
crates/world_runtime/src/economy.rs
crates/world_runtime/src/game.rs
crates/world_runtime/src/lib.rs
crates/world_runtime/src/navigation.rs
crates/world_runtime/src/protocol.rs
crates/world_runtime/src/quests.rs
crates/world_runtime/src/society.rs
crates/world_runtime/src/storage.rs
crates/world_runtime/src/streaming.rs
crates/world_runtime/src/terrain.rs
crates/world_runtime/src/types.rs
crates/world_runtime/src/weather.rs
crates/world_runtime/tests/acceptance.rs
crates/world_view/Cargo.toml
crates/world_view/src/audio.rs
crates/world_view/src/lib.rs
crates/world_view/tests/read_only.rs
docs/ARCHITECTURE.md
docs/CONTENT_RUNTIME.md
docs/CONTENT_SCHEMA.md
docs/DESIGN.md
docs/IMPLEMENTATION_PLAN.zh-CN.md
docs/PROTOCOL.md
docs/PROVENANCE.md
docs/REQUIREMENTS.zh-CN.md
docs/SAVE_FORMAT.md
docs/SOURCE_STATUS.zh-CN.md
docs/VALIDATION.md
docs/archive/m1a/AGENTS.md
docs/archive/m1a/IMPLEMENTATION_PLAN.zh-CN.md
docs/archive/m1a/PROVENANCE.md
docs/archive/m1a/README.md
docs/archive/m1a/VALIDATION.md
examples/hydrology.lpw
examples/market.lpw
examples/smoke.lpw
verification/archive/m1a/changes_from_m0.json
verification/archive/m1a/reference_checks.json
verification/archive/m1a/static_checks.json
verification/changes_from_m1a.json
verification/source_checks_v0_3.json
```

## Completion criterion

Finish uploading the missing original files through an authorized connection, remove this transfer-only status file, and verify the resulting Git tree against the full source tree above before updating `main`. Do not mark this import complete based only on file counts.
