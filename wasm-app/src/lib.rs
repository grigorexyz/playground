//! A minimal Rust + WebAssembly demo, inspired by the software-rendering
//! spirit of [tsoding/koil](https://github.com/tsoding/koil) and the
//! lightweight, no-frills project layout of `comet`-style wasm templates.
//!
//! It uses the idiomatic `wasm-bindgen` + `web-sys` stack: no hand-rolled
//! JS glue, no raw pointer plumbing — just a `#[wasm_bindgen(start)]` entry
//! point that drives a `requestAnimationFrame` loop and paints a simple
//! animated plasma effect onto a 2D canvas via `ImageData`.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Renders one frame of the plasma effect into `buffer` (RGBA8, row-major)
/// for the given canvas dimensions and elapsed time (in seconds).
fn render_frame(buffer: &mut [u8], width: u32, height: u32, time: f64) {
    let width = width as usize;
    let height = height as usize;

    for y in 0..height {
        for x in 0..width {
            let fx = x as f64 / width as f64;
            let fy = y as f64 / height as f64;

            let v = ((fx * 10.0 + time).sin()
                + (fy * 10.0 + time * 1.3).sin()
                + ((fx + fy) * 10.0 + time * 0.7).sin())
                / 3.0;

            let r = (128.0 + 127.0 * v).clamp(0.0, 255.0) as u8;
            let g = (128.0 + 127.0 * (v + fx).sin()).clamp(0.0, 255.0) as u8;
            let b = (128.0 + 127.0 * (v + fy).cos()).clamp(0.0, 255.0) as u8;

            let idx = (y * width + x) * 4;
            buffer[idx] = r;
            buffer[idx + 1] = g;
            buffer[idx + 2] = b;
            buffer[idx + 3] = 255;
        }
    }
}

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

/// Entry point invoked automatically once the wasm module is instantiated.
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook();

    let document = window().document().expect("no `document` on `window`");
    let canvas = document
        .get_element_by_id("wasm-canvas")
        .expect("no element with id `wasm-canvas`")
        .dyn_into::<HtmlCanvasElement>()?;

    let context = canvas
        .get_context("2d")?
        .expect("canvas has no 2d context")
        .dyn_into::<CanvasRenderingContext2d>()?;

    let width = canvas.width();
    let height = canvas.height();
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::new(move |time_ms: f64| {
        render_frame(&mut buffer, width, height, time_ms / 1000.0);

        let data =
            web_sys::ImageData::new_with_u8_clamped_array_and_sh(Clamped(&buffer), width, height)
                .expect("failed to build ImageData");
        context
            .put_image_data(&data, 0.0, 0.0)
            .expect("failed to paint ImageData");

        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

/// Installs a panic hook that forwards Rust panics to the browser console,
/// which is invaluable for debugging wasm without a native backtrace.
fn console_error_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("{info}").into());
    }));
}
