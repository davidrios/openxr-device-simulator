# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

An OpenXR runtime simulator written in Rust that emulates an HMD. It compiles as a `cdylib` shared library (`libopenxr_device_simulator_runtime.so`) that the OpenXR loader can discover and use, allowing OpenXR applications to run without physical VR hardware.

## Build Commands

```bash
# Build the runtime shared library
cargo build
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy

# Format
cargo fmt
```

The runtime manifest at `runtime_json/linux_debug.json` points the OpenXR loader to the built `.so`.

### Web Client (TypeScript/Vue/Quasar)

```bash
cd web_client
pnpm install
pnpm dev        # Dev server with hot-reload (connects to runtime on localhost:3050)
pnpm build      # Production build
pnpm lint       # ESLint
pnpm format     # Prettier
```

## Architecture

### Rust Workspace

- **`runtime/`** — Core cdylib implementing the OpenXR C ABI. This is the main crate.
- **`client/`** — Stub crate, minimal dependencies.
- **`vulkan-experiments/`** — Standalone Vulkan rendering sandbox, independent of the runtime.
- **`web_client/`** — Vue 3 + Quasar dashboard for monitoring/controlling the running simulator.

### Runtime Module Layout (`runtime/src/`)

The runtime exposes `xrNegotiateLoaderRuntimeInterface` which hands the OpenXR loader a dispatch table of function pointers. All public XR API functions are registered there in `lib.rs`.

Key areas:
- **`loader.rs`** — Loader negotiation protocol, function pointer binding, logging init
- **`instance/`** — `xrCreateInstance`, instance state machine, properties
- **`session.rs`** — Session lifecycle (create → begin → [running] → end → destroy)
- **`rendering/`** — Frame timing (`xrWaitFrame`/`xrBeginFrame`/`xrEndFrame`), swapchain management
- **`input/`** — Action sets, actions, input state polling (bool/float/vector2f/pose)
- **`spaces/`** — Reference and action space creation and location queries
- **`vulkan.rs`** — Vulkan device/extension requirements for OpenXR graphics binding
- **`server.rs`** — Axum + Socket.IO server on `localhost:3050` for web client communication
- **`error.rs`** — `Error` type with `From<Error> -> xr::Result` conversions

### Key Patterns

- **`bind_api_fn!` macro** (`utils.rs`) — Converts Rust functions to C function pointers for the OpenXR ABI.
- **Object storage** — `LazyLock<Mutex<HashMap<...>>>` keyed by opaque XR handles (instances, sessions, swapchains, spaces).
- **Helper closures** — `with_instance()`, `with_session()`, etc. provide ergonomic locked access to stored objects.
- **Server communication** — `crossbeam` bounded channels bridge the synchronous XR runtime thread to the async Tokio socket server.

### Web Client

Vue 3 + Quasar + Pinia + Socket.io-client. Two main pages:
- `ConnectPage.vue` — Address input (default `http://localhost:3050`), initiates Socket.IO connection
- `MainPage.vue` — Control panel, displays frame events, ping button

Connection state lives in `stores/connection.ts` (Pinia store).
