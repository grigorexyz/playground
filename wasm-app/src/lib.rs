//! Bootstrap for the WebAssembly-simulated desktop.
//!
//! `index.html` contains nothing but a `<canvas>` element; every pixel on
//! screen — window chrome, the taskbar, and the `CometApp` game running
//! inside it — is rendered from Rust through the WebGPU (`wgpu`)
//! pipeline in [`app::gpu`]. `main.js` only calls the exported `start`
//! entry point; there is no other JavaScript or CSS involved beyond
//! making the canvas fill the viewport.
//!
//! [`comet`] holds the game rules and has no `wasm-bindgen`/`wgpu`
//! dependency at all, so it (and its tests) build and run on any target;
//! everything else here only makes sense compiled for the browser.

mod comet;

#[cfg(target_arch = "wasm32")]
mod app;
