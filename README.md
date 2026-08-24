# Playground

A Rust + WebAssembly prototype of the **Comet** shedding card game
(https://qlayground.net/games/view.php?slug=comet), played against a
simple AI opponent in the browser.

## Rules implemented

- A standard 52-card deck (ranks 2–Ace, four suits) is dealt evenly
  between you and the AI (26 cards each), with the 9 of Diamonds
  replaced by the wildcard **Comet** card.
- Whoever holds the lowest card leads the first run.
- Players alternate extending an ascending run of any suit; if you
  can't beat the current rank (and don't hold the Comet), your turn is
  skipped.
- The Comet can be played on any rank and simply raises the run's
  required rank by one.
- Playing a King ends the run immediately; that same player then leads
  a fresh run.
- If neither player can continue a run, it dies and whoever played
  last starts a new one.
- First to empty their hand wins; emptying it with the Comet as the
  final card is a bonus win.

The game engine lives in [`wasm-app/src/comet.rs`](./wasm-app/src/comet.rs)
and is unit-tested with `cargo test`. The Rust crate in
[`wasm-app/`](./wasm-app) uses the idiomatic
[`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen) +
[`web-sys`](https://docs.rs/web-sys) stack (built with
[`wasm-pack`](https://github.com/rustwasm/wasm-pack)) to render the game
directly into the DOM — no canvas, just clickable card buttons and a
running log.

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

Run the game logic's unit tests with:

```sh
cd wasm-app && cargo test
```

## Deployment

Pushes to `master` trigger [`.github/workflows/pages.yml`](.github/workflows/pages.yml),
which builds the wasm package and deploys the site to GitHub Pages.

