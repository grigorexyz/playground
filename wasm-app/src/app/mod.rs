mod apps;
mod desktop;
mod font;
mod geometry;
mod gpu;
mod painter;

use std::cell::RefCell;
use std::rc::Rc;

use apps::comet_app::CometApp;
use desktop::Desktop;
use geometry::Rect;
use gpu::Renderer;
use painter::Painter;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn document() -> web_sys::Document {
    window().document().expect("no `document` on `window`")
}

/// Installs a panic hook that forwards Rust panics to the browser
/// console, invaluable for debugging wasm without a native backtrace.
fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("{info}").into());
    }));
}

struct State {
    renderer: Renderer,
    desktop: Desktop,
    painter: Painter,
    canvas: HtmlCanvasElement,
}

type Shared = Rc<RefCell<State>>;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    set_panic_hook();

    let canvas: HtmlCanvasElement = document()
        .get_element_by_id("desktop")
        .expect("index.html must contain a <canvas id=\"desktop\">")
        .dyn_into()?;

    wasm_bindgen_futures::spawn_local(async move {
        if let Err(err) = boot(canvas).await {
            web_sys::console::error_1(&err);
        }
    });

    Ok(())
}

fn canvas_size(canvas: &HtmlCanvasElement) -> (f32, f32) {
    (canvas.client_width().max(1) as f32, canvas.client_height().max(1) as f32)
}

async fn boot(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    let (width, height) = canvas_size(&canvas);
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);

    let renderer = match Renderer::new(canvas.clone(), width as u32, height as u32).await {
        Ok(renderer) => renderer,
        Err(reason) => {
            show_unsupported_message(&reason)?;
            return Ok(());
        }
    };

    let mut desktop = Desktop::new((width, height));
    desktop.open_window(
        Box::new(CometApp::new()),
        Rect::new(24.0, 24.0, 420.0, 480.0),
    );

    let state: Shared = Rc::new(RefCell::new(State {
        renderer,
        desktop,
        painter: Painter::new(),
        canvas: canvas.clone(),
    }));

    install_input_handlers(&state);
    install_resize_handler(&state);
    start_frame_loop(state);
    Ok(())
}

fn pointer_coords(state: &Shared, event: &web_sys::PointerEvent) -> (f32, f32) {
    let rect = state.borrow().canvas.get_bounding_client_rect();
    (event.client_x() as f32 - rect.left() as f32, event.client_y() as f32 - rect.top() as f32)
}

fn install_input_handlers(state: &Shared) {
    let canvas = state.borrow().canvas.clone();

    {
        let state = state.clone();
        let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |event| {
            let (x, y) = pointer_coords(&state, &event);
            state.borrow_mut().desktop.pointer_down(x, y);
        });
        canvas
            .add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref())
            .expect("failed to attach pointerdown listener");
        closure.forget();
    }
    {
        let state = state.clone();
        let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |event| {
            let (x, y) = pointer_coords(&state, &event);
            state.borrow_mut().desktop.pointer_move(x, y);
        });
        canvas
            .add_event_listener_with_callback("pointermove", closure.as_ref().unchecked_ref())
            .expect("failed to attach pointermove listener");
        closure.forget();
    }
    {
        let state = state.clone();
        let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |_event| {
            state.borrow_mut().desktop.pointer_up();
        });
        canvas
            .add_event_listener_with_callback("pointerup", closure.as_ref().unchecked_ref())
            .expect("failed to attach pointerup listener");
        closure.forget();
    }
}

fn install_resize_handler(state: &Shared) {
    let state = state.clone();
    let closure = Closure::<dyn FnMut()>::new(move || {
        let mut state = state.borrow_mut();
        let (width, height) = canvas_size(&state.canvas);
        state.canvas.set_width(width as u32);
        state.canvas.set_height(height as u32);
        state.renderer.resize(width as u32, height as u32);
        state.desktop.set_size((width, height));
    });
    window()
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        .expect("failed to attach resize listener");
    closure.forget();
}

/// `Rc<RefCell<Option<...>>>` trick so the `requestAnimationFrame`
/// callback can re-schedule itself.
fn start_frame_loop(state: Shared) {
    let callback: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let callback_slot = callback.clone();

    *callback.borrow_mut() = Some(Closure::new(move |now_ms: f64| {
        {
            let mut guard = state.borrow_mut();
            let State { renderer, desktop, painter, .. } = &mut *guard;
            desktop.update(now_ms);
            painter.clear();
            desktop.render(painter);
            let clear = wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
            if let Err(err) = renderer.render(&painter.quads, clear) {
                web_sys::console::error_1(&err);
            }
        }
        window()
            .request_animation_frame(
                callback_slot.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
            )
            .expect("requestAnimationFrame failed");
    }));

    window()
        .request_animation_frame(callback.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .expect("requestAnimationFrame failed");
}

/// Drawn as a plain DOM message (the one place this project falls back to
/// raw HTML) when the browser has no WebGPU support at all, since at that
/// point there's no GPU surface left to draw an in-canvas message with.
fn show_unsupported_message(reason: &str) -> Result<(), JsValue> {
    let doc = document();
    if let Some(canvas) = doc.get_element_by_id("desktop") {
        canvas.set_attribute("hidden", "true")?;
    }
    let message = doc.create_element("p")?;
    message.set_id("webgpu-unsupported");
    message.set_text_content(Some(&format!(
        "This desktop needs a browser with WebGPU support: {reason}"
    )));
    doc.body().expect("document has no body").append_child(&message)?;
    Ok(())
}
