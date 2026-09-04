# Session handoff — 2026-09-04 (continued)

For a new Claude Code session picking this up. Read `CLAUDE.md` first for the architecture; this doc is "what happened, what's next."

## State of the repo

- Branch: `main`.
- Latest commit: `a7341a9` "Default head/hand pose to standing height, not floor level" — check `git status`/`git log origin/main..main` on resume to confirm it's pushed (it was pushed at the time this doc was written).
- Working tree was clean as of this doc being written.
- `vulkan-validation-layers` is now installed system-wide (Arch package `vulkan-validation-layers`). To use it: set `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation` in the environment of whatever OpenXR app you run against the runtime (the app creates its own Vulkan instance, so this must be set on the app's process, not the runtime).

## What got done this session (continuing from the prior handoff)

Picked up the "chase the bevy_oxr missing-geometry bug" suggested next step and fully closed it out — it turned out to be two separate things, one real runtime bug and one non-bug default-state issue.

Installed Vulkan validation layers, built `bevy_oxr`'s `3d_scene` example natively (NTFS mount still can't run binaries in place — build with `CARGO_TARGET_DIR` pointing at a native path, e.g. under the scratchpad), and used Playwright's browser tools (the user declined the Claude-in-Chrome extension this session, use `mcp__playwright__*` instead) to connect the web client and screenshot actual output for comparison, rather than trusting logs alone.

**Bug #1 (real, fixed):** Comparing the two stereo panes side-by-side in a screenshot turned up something real: the right-eye pane was always 100% black with no content at all — not even the hand-tracking gizmos that rendered correctly in the left-eye pane. Root-caused to `runtime/src/rendering/swapchain.rs::OffscreenImage::read_pixels`: the staging buffer was correctly sized for `array_layers` worth of pixel data, but both the layout-transition barrier and the actual `vkCmdCopyImageToBuffer` region hardcoded `layer_count: 1`. So for array/multiview stereo swapchains (bevy_oxr's approach — one image, one layer per eye), only array layer 0 was ever actually copied off the GPU; every other layer's bytes in the returned buffer were whatever was already in the freshly-allocated staging memory (in practice, zeroed/black). Fixed in commit `40da66b` by changing both hardcoded `layer_count: 1`s to `self.array_layers`. Verified live (right eye now shows real content matching the left eye's stereo parallax) and regression-tested against `hello_xr`'s per-eye-swapchain path (no change in behavior there, `array_layers` is always 1 for it).

Validation layers themselves turned up nothing pointing at the missing geometry — just pre-existing shader/SPIR-V warnings from bevy's own wgpu pipeline creation (`VUID-VkPipelineShaderStageCreateInfo-flags-parameter`, `VUID-StandaloneSpirv-MemorySemantics-10871`, `VUID-StandaloneSpirv-None-10684`), which look like known wgpu/naga-vs-validation-layer false positives, not runtime bugs.

**"Bug" #2 (not actually a bug, closed):** After fixing the readback, the cube/plane still didn't render in *either* eye — just a solid black background, in both eyes now. Chased this hard: tried disabling MSAA (bevy defaults to `Msaa::Sample4`, a per-camera `Component` not a `Resource` in bevy 0.16 — `insert_resource(Msaa::Off)` doesn't even compile, had to add the component directly to the XR camera spawn in `bevy_openxr`'s `render.rs`), waited 20+ seconds in case of pipeline-compile latency, read through `bevy_mod_xr`'s `XrProjection`/`calculate_projection` (well-tested, matches the standard OpenXR asymmetric-FOV formula) and noticed `bevy_openxr`'s `render.rs` puts `NoFrustumCulling` on the *camera* entity, which is actually a per-*mesh* opt-out marker in bevy — probably ineffective, though this turned out to be a red herring too.

The user then mentioned, mid-investigation, that they'd tested this exact `bevy_oxr` demo on a real Meta Quest 3 and it worked — and that they'd felt like they were standing *inside* the cube, head slightly above it. That was the key clue: **this runtime's default head pose (`device_state.rs`'s `DeviceState::default()`) started at literal world origin `(0,0,0)` — floor level in the STAGE space — coincident with the example's floor plane and inside the cube (centered at `y=0.5`)**. Confirmed by scripting the web client's WASD/Space controls via Playwright (`document.dispatchEvent(new KeyboardEvent(...))` — note events must be dispatched on `document`, not `window`, to reach the page's listeners) to raise and back the viewpoint away before screenshotting: the cube, floor, shadow, and lighting all rendered correctly. Not a runtime bug at all — the rendering pipeline has been fine the whole time.

Fixed by changing the default standing height from floor level to `1.6`m in **both** `runtime/src/input/device_state.rs`'s `DeviceState::default()` (matters before any web client connects) **and** `web_client/src/pages/MainPage.vue`'s `position`/`leftHandPos`/`rightHandPos` defaults (matters in practice, since the client overwrites the runtime's default the instant it connects and starts streaming input every animation frame — controller/head poses are sent as absolute values, not composed/relative to each other in the protocol). Verified live: connecting fresh with no manual input now shows the floor immediately instead of solid black (the cube itself may or may not be in the exact forward-facing default view — that's just normal "look around a room" VR behavior, not a bug). Commit `a7341a9`.

`CLAUDE.md`'s "Known gaps" and `docs/device-sim-plan.md`'s "Related but separate" section were both updated to reflect the real bug (fixed) and the closed non-bug (also fixed, as a UX default improvement).

## Known open issues

None from this investigation — it's fully closed out. Remaining gaps are the pre-existing ones in `CLAUDE.md`'s "Known gaps": no real 6DoF controller tracking (keyboard-driven X/Y plane only; Phase 3 WebHID/Gamepad API work is unimplemented).

## Running processes

None left running at the end of this session — `bevy_oxr`'s `3d_scene` and `hello_xr` were both killed after their test runs. The `web_client` dev server (`pnpm dev`, `http://localhost:9000`) was still running throughout (with HMR, so it already picked up the `MainPage.vue` change) and is very likely still alive — check with `ps aux | grep "quasar dev"` before assuming.

To restart testing: see "Testing against real apps" in `CLAUDE.md`. Remember `bevy_oxr` must be built with `CARGO_TARGET_DIR` pointing somewhere native (not the NTFS mount) — this session used a path under the Claude scratchpad dir, which won't persist across sessions, so rebuild rather than looking for the old binary.

## Suggested next steps (pick one)

1. **Continue the device-sim plan**: Phase 3 (real gamepad/WebHID support — needs the user's own DualSense/Joy-Con hardware and a real browser, can't be fully automated).
2. Poke at whether `bevy_openxr`'s `NoFrustumCulling`-on-camera-not-mesh placement (noted above, in `crates/bevy_openxr/src/openxr/render.rs`'s `init_views`) is worth upstreaming a fix for — it looked ineffective by reading bevy's own doc comment for that marker, though it never actually caused an observed problem in this session, so it's speculative, not confirmed.
3. Something else the user asks for — this repo has consistently rewarded testing against real apps in a real browser, comparing both eyes side-by-side, and (this session) taking the user's own hardware experience as a real data point, over trusting API-level logs or first impressions alone.
