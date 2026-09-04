# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

An OpenXR runtime simulator written in Rust that emulates an HMD (goal: a "Meta Quest 3 on your desk" — see `docs/device-sim-plan.md`). It compiles as a `cdylib` shared library (`libopenxr_device_simulator_runtime.so`) that the OpenXR loader can discover and use, allowing OpenXR applications to run without physical VR hardware. Rendered frames stream to a web dashboard (`web_client/`) over Socket.IO, and the dashboard's mouse/keyboard controls stream back as head + controller input.

## Build Commands

```bash
# Build the runtime shared library (this is what you use day to day — see "Dev build performance" below)
cargo build

# Full workspace build/test
cargo build --workspace
cargo test --workspace

# Lint
cargo clippy

# Format
cargo fmt
```

The runtime manifest at `runtime_json/linux_debug.json` points the OpenXR loader to `target/debug/libopenxr_device_simulator_runtime.so`. There is no deployed `--release` workflow — see "Dev build performance" below for why.

### Web Client (TypeScript/Vue/Quasar)

```bash
cd web_client
pnpm install
pnpm dev        # Dev server with hot-reload (connects to runtime on localhost:3050), usually http://localhost:9000
pnpm build      # Production build
pnpm lint       # ESLint
pnpm format     # Prettier
```

## Architecture

### Rust Workspace

- **`runtime/`** — Core cdylib implementing the OpenXR C ABI. This is the main crate.
- **`client/`** — Stub crate, minimal dependencies.
- **`vulkan-experiments/`** — Standalone Vulkan rendering sandbox, independent of the runtime.
- **`web_client/`** — Vue 3 + Quasar dashboard: shows the two eye views live and drives the simulated device via mouse/keyboard.

### Runtime Module Layout (`runtime/src/`)

The runtime exposes `xrNegotiateLoaderRuntimeInterface` which hands the OpenXR loader a dispatch table of function pointers. All public XR API functions are registered there in `lib.rs`.

Key areas:
- **`loader.rs`** — Loader negotiation protocol, function pointer binding, logging init, starts the socket server
- **`instance/`** — `xrCreateInstance`, instance state machine, properties, supported extensions
- **`session.rs`** — Session lifecycle (create → begin → [running] → end → destroy). `check_ready()` gates `IDLE → READY` on the app having created a space + swapchain — **not** on action sets, since those are optional in OpenXR
- **`rendering/`** — Frame timing (`xrWaitFrame`/`xrBeginFrame`/`xrEndFrame`, `frame.rs`), swapchain management + GPU readback + JPEG encode (`swapchain.rs`), view/eye pose computation (`view.rs`)
- **`input/`** — `device_state.rs` (the single live-state source of truth: head pose + per-hand controller pose/buttons, updated by the web client), `action.rs`/`action_set.rs`/`action_state.rs` (real `xrSyncActions` resolving action values from `device_state` via whatever interaction-profile bindings the app suggested), `hand_tracking.rs` (mock hand joints, driven by controller pose + grip/trigger)
- **`spaces/`** — Reference and action space creation and location queries; action spaces resolve to the live synced action pose, VIEW reference space resolves to the live head pose
- **`vulkan.rs`** — Vulkan device/extension requirements for OpenXR graphics binding
- **`server.rs`** — Axum + Socket.IO server on `localhost:3050` for web client communication. Only one client connection is allowed at a time (`IS_CONNECTED` gate)
- **`error.rs`** — `Error` type with `From<Error> -> xr::Result` conversions

### Key Patterns

- **`bind_api_fn!` macro** (`utils.rs`) — Converts Rust functions to C function pointers for the OpenXR ABI.
- **Object storage** — `LazyLock<Mutex<HashMap<...>>>` keyed by opaque XR handles (instances, sessions, swapchains, spaces).
- **Helper closures** — `with_instance()`, `with_session()`, etc. provide ergonomic locked access to stored objects.
- **Server communication** — `crossbeam` bounded channels bridge the synchronous XR runtime thread to the async Tokio socket server.
- **Math helpers** (`utils.rs`) — `rotate_vec3`/`mul_quat`/`compose_poses`, shared by `rendering/view.rs` and `spaces/mod.rs` — don't duplicate these locally.
- **Swapchains can be per-eye OR array-layer stereo.** Some apps (`hello_xr`) create two swapchains, one per eye. Others (`bevy_oxr`) create one array swapchain with one layer per eye. The runtime supports both — `dump_frame` in `swapchain.rs` splits a multi-layer GPU readback into one JPEG per layer, and the wire protocol/web client key frames by `(swapchain_id, layer)`, not just `swapchain_id`.

### Web Client

Vue 3 + Quasar + Pinia + Socket.io-client. Two main pages:
- `ConnectPage.vue` — Address input (default `http://localhost:3050`), initiates Socket.IO connection
- `MainPage.vue` — The cockpit: both eye views rendered side-by-side and **swapped** for cross-eyed free-viewing (right eye on screen-left), a size slider (bigger previews are harder to cross-eye-fuse). Controls: mouse-look for head orientation; plain `WASD` drives the left-hand movement joystick (thumbstick) for the app's own locomotion to read, while **Shift** is a modifier — `Shift+WASD` moves the head instead, `Space`/`Ctrl` move it up/down unconditionally. `IJKL` (left hand) and arrow keys (right hand) aim that hand (yaw/pitch, for pointing a laser pointer at UI) by default, and move it instead when held with Shift. Left hand: `R` squeeze, `F` trigger, `1-2-3-4` = A/B/stick-click/menu. Right hand: `RCtrl` squeeze, `Enter` trigger, `7-8-9-0` = A/B/stick-click/menu — see the in-page help icon for the full legend. `q-page` only gets a `min-height` from Quasar in this app, not a real height, so the page measures the actual available height in JS (`availableHeight`/`updateAvailableHeight`) rather than relying on flex-grow-fills-viewport, which silently doesn't work here.

Connection state lives in `stores/connection.ts` (Pinia store) — `sendInput()` sends `{ head, left_hand, right_hand }` (full pose + buttons per hand) every animation frame; `frames` is keyed by `"<swapchain_id>:<layer>"`.

## Dev build performance

`cargo build` (not `--release`) is what's used day to day, but plain `opt-level=0` makes the per-frame JPEG encode + GPU pixel readback (`rendering/swapchain.rs`) slow enough to dominate frame time (~2 FPS). `Cargo.toml` sets `[profile.dev] opt-level = 1` plus `opt-level = 3` overrides for `image`/`zune-jpeg`/`base64` (the actual hot path) — this gets near-release runtime performance while keeping normal debug builds fast to iterate on and fully debug-assertion-checked. Don't "helpfully" switch to `--release` for perf; the overrides already get most of the benefit, and a full `--release` build was in fact broken until a UB bug in `event.rs` got fixed (see git log "Fix a real UB bug...").

## Testing against real apps

The runtime is a C ABI shared library — you need a real OpenXR app to drive it, not just `cargo test` (there are no unit tests; correctness is verified by running real apps against it).

**`hello_xr`** (OpenXR-SDK-Source sample) is at `~/work/OpenXR-SDK-Source/build/linux_debug/src/tests/hello_xr/hello_xr`. Run:
```bash
XR_RUNTIME_JSON=/home/david/work/openxr-device-simulator/runtime_json/linux_debug.json \
  hello_xr -g Vulkan2 -s Local < <(sleep infinity)
```
The `< <(sleep infinity)` is required in non-interactive shells: `hello_xr` spawns a thread that blocks on `getchar()` and quits the whole app on ANY stdin activity including EOF — a closed/non-interactive stdin looks like "a key was pressed" and kills it before the render loop ever starts. It suggests bindings for `/interaction_profiles/oculus/touch_controller` among others — good for testing controller input.

**`bevy_oxr`** (a real game engine, more demanding than `hello_xr`) is at `/mnt/shared/work/bevy_oxr`. That path is an **NTFS mount** — the Linux `ntfs3` driver can't reliably read large files there (even `cp` fails with `Invalid argument`, not just exec), so build with `CARGO_TARGET_DIR` pointing somewhere native (e.g. `/tmp/...`) rather than trying to run prebuilt binaries from `target/` in place. `crates/bevy_openxr` (package `bevy_mod_openxr`) has runnable examples, e.g. `3d_scene`.

**`godot-xr-template`** (a real Godot game — https://github.com/godotVR/godot-xr-template, needs Godot 4.6+) exercises the runtime differently from the above two and has driven most of the input/session-lifecycle bugfixes: it creates action sets before any session exists, creates a "probe" session before the real one, waits for `READY` before ever creating a swapchain, hard-requires a usable depth swapchain format to initialize at all, and won't poll any non-pose action state until `xrGetCurrentInteractionProfile` succeeds. A Godot binary isn't checked into this repo; download one (e.g. from godotengine.org) and import the project once (`godot --headless --import --path <project>`) before running it for real. It defaults to the `gl_compatibility` (OpenGL) rendering method, which needs `XR_KHR_opengl_enable` — this runtime is Vulkan-only, so force Vulkan without touching the project file:
```bash
XR_RUNTIME_JSON=/home/david/work/openxr-device-simulator/runtime_json/linux_debug.json \
  godot --path <path-to-godot-xr-template> --rendering-method forward_plus --rendering-driver vulkan
```

Set `RUST_LOG=debug` (or `info,openxr_device_simulator_runtime=debug`) to see the runtime's own logging.

## Known gaps

- Controller position is keyboard-driven (X/Y plane) via the web client, not real 6DoF — real gamepad/gyro support (PS5 DualSense / Switch Joy-Con via Gamepad API + WebHID) is planned but not implemented.
