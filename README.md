# Playground

A minimal Rust + WebAssembly demo, inspired by the software-rendering
spirit of [tsoding/koil](https://github.com/tsoding/koil) and the
lightweight, no-frills layout of `comet`-style wasm templates.

The Rust crate in [`wasm-app/`](./wasm-app) uses the idiomatic
[`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen) +
[`web-sys`](https://docs.rs/web-sys) stack (built with
[`wasm-pack`](https://github.com/rustwasm/wasm-pack)) to drive a
`requestAnimationFrame` loop that paints an animated plasma effect onto a
2D canvas.

## Building locally

Prerequisites: a Rust toolchain with the `wasm32-unknown-unknown` target,
and [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/).

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build wasm-app --target web --release --out-dir ../pkg
```

Then serve the repository root and open `index.html`, e.g.:

```sh
python3 -m http.server
```

## Deployment

Pushes to `master` trigger [`.github/workflows/pages.yml`](.github/workflows/pages.yml),
which builds the wasm package and deploys the site to GitHub Pages.
