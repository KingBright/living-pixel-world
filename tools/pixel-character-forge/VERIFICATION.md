# Verification record: v0.2

Date: 2026-09-07.

## Local environment

The editing container has no Rust compiler or Cargo. Toolchain download could not resolve its network host. Consequently, no local Rust compilation, unit-test execution, desktop startup or real-renderer screenshot review has been performed. Static inspection is not a compiler, and test source is not evidence of passing tests.

## Implemented validation suite

38 integration tests are present: 15 existing core tests plus 23 tests for editable clips, undo/redo, interpolation, retiming, mirror/inverse transforms, one-shot endpoints, fixed-step replay, IK invariants, shared grips, real high-resolution rasterization, outline behavior, detail toggles, side views and paged export. Execution status: not yet confirmed.

JSON/TOML parsing, archive integrity and local/remote source hashes are checked during packaging. GitHub Actions is supplied to execute the Rust suite and produce actual renderer images. Consult the branch's workflow result before treating the build as verified.

## Scope of evidence

No fabricated screenshot or AI-rendered illustration is supplied as engine output. A successful headless export would demonstrate that the renderer runs, not that interactive editing, performance, every wardrobe combination or Windows/macOS behavior has been validated.

## Manual acceptance still required

Open the desktop app; compare 128/192/256/384/512, None/SoftSilhouette/LegacyInk, and Fine detail. Check faces, sleeves, boots, cape and hair through all directions and moving poses. Edit a key by dragging a hand/foot, retime it, undo/redo, save/load, then compare the exported custom clip. Test double-handed weapon grips and one-shot final poses. Check window resizing and CPU load with face inset/onion skins enabled.
