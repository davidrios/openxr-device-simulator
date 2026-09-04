# Quest 3-style device simulation: live HMD + controller/hand input

> Status as of 2026-09-04 (see `HANDOFF.md` at repo root for the full session summary). Phases 1 and 2 are done and live-verified. Phase 5's cockpit UI got a head start (compact status bar, cross-eyed view, size slider) while polishing phases 1-2, out of the original order. Phases 3 and 4 are not started.

## Context

The runtime already does the hard part: it renders real OpenXR app frames offscreen via Vulkan, JPEG-streams them to `web_client` over Socket.IO, and lets the mouse rotate the simulated head. Everything else about "the device" was originally a stub — this doc is the plan for turning it into a real "virtual Quest 3 on your desk": a web cockpit showing the HMD screen, with the HMD and both hands/controllers driveable by mouse+keyboard, and optionally by a real PS5 DualSense or Switch Joy-Con held in your hands (buttons/sticks via the standard Gamepad API, gyro orientation via WebHID).

This is a big feature, phased on purpose. Each phase is independently useful and buildable/testable on its own — implement and verify one phase at a time.

## Architecture overview

**One source of truth**: `runtime/src/input/device_state.rs`'s `DeviceState { head: Posef, hands: [HandState; 2] }`, where `HandState { controller_pose: Posef, buttons: ControllerButtons }`. Guarded by a `LazyLock<Mutex<DeviceState>>`, `get_device_state()` / `set_head_pose()` / `set_hand()`. This is the single thing the web client updates (via the `input` Socket.IO event) and the single thing the runtime reads from (`view.rs`, `spaces/mod.rs`, `action_state.rs`).

## Phase 1 — Full 6DOF head movement ✅ done (commit `a5f86aa` on the old `try_render_web` branch, now on `main`)

Head position + orientation, not just yaw/pitch. `device_state.rs` replaced the old `HEAD_LOOK` static; `view.rs` and `spaces/mod.rs` (VIEW reference space) both read live head pose; web client sends full pose via `requestAnimationFrame`, WASD + Space/Shift added for position on top of the existing mouse-look.

## Phase 2 — Live controller input through the real OpenXR action-binding pipeline ✅ done (commit `7d86af9`)

`xrSyncActions` (`runtime/src/input/action_state.rs`) now actually resolves each action's subaction values from `DeviceState` via whatever interaction-profile bindings the app suggested (`instance.interaction_profile_bindings`), preferring `/interaction_profiles/oculus/touch_controller` (`TARGET_INTERACTION_PROFILE`) when an app suggests multiple profiles for compatibility (a real bug — naively merging picked an incompatible binding — was caught and fixed here via live testing). Action spaces (grip/aim pose) resolve to the live synced pose. Web client: keyboard-driven controllers — right: arrows (stick)/RCtrl (squeeze)/RShift (trigger)/7-8-9-0 (A/B/stick-click/menu); left: IJKL/R/F/1-2-3-4. **Controller position is X/Y-plane keyboard-driven, not real 6DoF** — that's Phase 3.

## Phase 3 — Real gamepad support (PS5 DualSense, Switch Joy-Con) — not started

Two separate browser APIs, layered as independent "input source" modules behind one interface (`web_client/src/services/input-sources/`), each just populating the same `DeviceState` shape as the mouse/keyboard source does:

- **Buttons + sticks — standard Gamepad API** (works in every browser, no permissions): `navigator.getGamepads()`, polled each `requestAnimationFrame`.
- **Gyro orientation — WebHID** (Chromium-only: Chrome/Edge/Opera; not Firefox/Safari — flag this in the UI). The standard Gamepad API has no motion data; DualSense/Joy-Con motion sensors are only reachable via raw HID reports. Use the open patterns from `jsDualsense`/`dualsense-ts` (DualSense) and the `joy-con-webhid` project (Joy-Con) as reference for report parsing — implement minimal parsers rather than pulling in the full libraries. Recommend/require USB connection (Sony's controllers default to a low-power BT mode that suppresses motion reports).
- **3DoF gyro → 6DoF controller pose**: gyro alone gives orientation, not position. Use the standard WebXR "3DoF-to-6DoF" arm-model trick: fix a virtual shoulder point relative to the current head pose, place the controller at `shoulder + armLength * forward(orientation)`. Simpler and more robust than double-integrating accelerometer data (drifts badly). A "recenter" button rezeros gyro yaw drift and the arm-model anchor.
- Web UI: a connect/pairing panel — Gamepad API devices show up automatically on button press; WebHID devices need an explicit `navigator.hid.requestDevice()` user-gesture button per controller. Show connection state and which hand each physical controller is assigned to.

**Verify:** manual test with real DualSense/Joy-Con hardware — buttons/sticks via Gamepad API should work immediately; WebHID gyro connects (Chrome only) and rotates the virtual controller.

## Phase 4 — Hands driven by the same state (+ controller/hand-tracking toggle) — not started

- `runtime/src/input/hand_tracking.rs`: replace the fixed `JOINT_OFFSETS`-from-static-wrist logic with joints computed from `DeviceState.hands[hand].controller_pose` plus a curl amount from `buttons.grip`/`buttons.trigger` (lerp open-palm vs loose-fist presets).
- UI toggle per hand: Controller vs Hand-tracking input mode, matching Quest 3's real behavior. Real OpenXR apps expect an `XR_TYPE_EVENT_DATA_INTERACTION_PROFILE_CHANGED` event on this kind of switch (`runtime/src/event.rs` already has an event queue/`schedule_event`) — wire that in if time allows, otherwise document as a known gap.

## Phase 5 — Cockpit UI pass — partially done ahead of schedule

Done while polishing phases 1-2 (see `HANDOFF.md`): compact single-row status bar (mouse-look/WASD hint, cross-eye hint, full key legend behind a hover-tooltip help icon instead of permanent text), the two eye previews rendered side-by-side and swapped for cross-eyed free-viewing, a size slider (bigger previews are harder to cross-eye-fuse — defaults to 60%, persisted to `localStorage`). Still open: active-input-source indicator per hand, a recenter button (relevant once Phase 3 adds gyro drift), and reconciling this with Phase 3/4's connect/pairing panel and mode toggle UI.

## Notes / things intentionally deferred

- No attempt to simulate Quest 3 passthrough/room-scale boundary — out of scope for a desktop dev simulator.
- Position from accelerometer integration is explicitly rejected in favor of the arm-model (drift-free, simpler, standard practice).
- Multi-controller-profile support (Touch Pro/Plus-specific extension) deferred past MVP; the core Oculus Touch profile covers the large majority of real apps.

## Related but separate: real-app compatibility fixes

While live-testing phases 1-2 and later running `bevy_oxr` (a real game engine, not just the `hello_xr` sample) against the runtime, several genuine pre-existing bugs were found and fixed (not part of this plan's scope, but relevant context for anyone touching these areas): `xrApplyHapticFeedback` was a hard error instead of an accepted no-op (crashed apps that use haptics); `session.rs::check_ready()` incorrectly required an attached action set to reach `READY` (action sets are optional in OpenXR); `view.rs::locate_views` rejected the standard two-call enumeration idiom; `swapchain.rs` hardcoded `array_layers: 1` regardless of what an app requested, corrupting array/multiview swapchains (one image, one layer per eye — `bevy_oxr`'s approach, vs `hello_xr`'s two-separate-swapchains approach). See `HANDOFF.md` and git log for details. **Known open gap:** `bevy_oxr`'s `3d_scene` example reaches a running session and streams distinct per-eye frames, but the actual PBR scene geometry doesn't render (black background instead of the lit cube/plane) — not yet root-caused, likely a depth-buffer or blend-mode gap.
