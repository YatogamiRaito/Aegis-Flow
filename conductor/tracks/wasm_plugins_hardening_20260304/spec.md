# Track Specification: WASM Plugin Engine Proxy Integration

## Overview
Connect the isolated Wasmtime `aegis-plugins` engine to the core `aegis-proxy`, exposing host ABI functions for WebAssembly modules to process incoming HTTP/3 requests and manipulate responses in real-time.

## Functional Requirements

### FR-1: WASM Host ABI Functions
- Expose shared memory read/write functions for `PluginRequest` and `PluginResponse`.
- Implement zero-copy passing where possible to adhere to string limits.

### FR-2: Example Rust-to-Wasm Plugin
- Write a reference plugin (e.g., Request Header Injector or Rate Limiter).
- Compile target must be `wasm32-unknown-unknown`.
- Ensure it successfully parses the ABI and mutating the `PluginResponse.modified_headers`.

### FR-3: Proxy Request Pipeline Hook
- Instantiate `PluginRegistry` within Aegis Proxy boot sequence.
- Add pre-routing and post-routing middleware hooks inside `crates/proxy/src/http3_handler.rs` or `crates/proxy/src/proxy.rs`.

### FR-4: Kusursuzluk Fazı (Perfection Elements)
- True Zero-copy memory allocator within the Wasm guest memory export.
- Guaranteed timeout limit (Fuel check validation under load) to prevent infinite WASM loop attacks.
- Structured concurrency so multiple requests do not stall the engine lock.

## Non-Functional Requirements

### NFR-1: Overhead Validation
- End-to-end plugin call MUST add < 100µs overhead compared to standard bare-metal routes.

### NFR-2: Hot-Reload Stress Test
- Replacing a .wasm file under 10k RPS load must successfully swap the plugin logic without dropping connections or crashing the engine.

## Acceptance Criteria
1. WebAssembly plugin successfully attaches a custom HTTP Header to an Aegis-Proxy response.
2. The plugin execution remains under the 100µs budget.
3. Overloading the plugin with an infinite loop yields a fast fail via Fuel consumption limits, keeping the proxy alive.
