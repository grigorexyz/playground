# Playground

A Rust + WebAssembly + **WebGPU** simulated desktop that runs entirely in
the browser: `index.html` is just a full-viewport `<canvas>`, and every
pixel on screen — window chrome, a taskbar, and the apps running inside
it — is drawn from Rust through [`wgpu`](https://github.com/gfx-rs/wgpu)
targeting WebGPU. There's no other HTML, CSS or JavaScript beyond making
that canvas fill the page and loading the wasm module.

The first (and so far only) app running on the desktop is **Comet**, a
shedding card game (https://qlayground.net/games/view.php?slug=comet)
played against a simple AI opponent.

## Architecture

- [`wasm-app/src/comet.rs`](./wasm-app/src/comet.rs) — the Comet rules
  engine. Pure Rust, no `wasm-bindgen`/`wgpu` dependency, unit-tested with
  `cargo test` and buildable/testable on any target.
- [`wasm-app/src/app/gpu.rs`](./wasm-app/src/app/gpu.rs) — the WebGPU
  backend: sets up the `wgpu` device/surface against the canvas and
  draws every rectangle on screen as one instanced-quad pipeline
  (see [`shader.wgsl`](./wasm-app/src/app/shader.wgsl)).
- [`wasm-app/src/app/font.rs`](./wasm-app/src/app/font.rs) /
  [`painter.rs`](./wasm-app/src/app/painter.rs) — a tiny hand-authored
  5x7 bitmap font and an immediate-mode "painter" that turns rectangles
  and text into GPU quad instances; there's no HTML/CSS text or
  canvas-2D/font-rendering crate involved.
- [`wasm-app/src/app/desktop.rs`](./wasm-app/src/app/desktop.rs) — a
  minimal window manager (draggable/closable windows, focus, a taskbar)
  and the `App` trait that pluggable programs implement.
- [`wasm-app/src/app/apps/comet_app.rs`](./wasm-app/src/app/apps/comet_app.rs) —
  wires the Comet engine into an `App`, rendering cards/log/status with
  the quad/text painter and turning clicks into `Game::play` calls.
- [`wasm-app/src/app/mod.rs`](./wasm-app/src/app/mod.rs) — bootstraps
  the canvas, initializes WebGPU asynchronously, wires up pointer/resize
  events, and drives the whole thing from a `requestAnimationFrame` loop.
  This module (and everything under `app/`) is only compiled for
  `wasm32` targets, so `cargo test` on the host still works.

### Browser support

WebGPU isn't available in every browser yet (recent Chrome/Edge have it;
Firefox/Safari support is still rolling out). If `navigator.gpu`/the
WebGPU adapter request fails, the app falls back to a plain text message
in the page instead of a blank canvas.

## Building locally

Prerequisites: a Rust toolchain with the `wasm32-unknown-unknown` target,
and [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/).

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build wasm-app --target web --release --out-dir ../pkg
```

Then serve the repository root and open `index.html` in a WebGPU-capable
browser, e.g.:

```sh
python3 -m http.server
```

Run the game logic's unit tests with:

```sh
cd wasm-app && cargo test
```

## Deployment

Pushes to `master` trigger [`.github/workflows/pages.yml`](.github/workflows/pages.yml),
which builds the wasm package and deploys the site to GitHub Pages.


