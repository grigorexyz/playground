//! A Rust + WebAssembly prototype of the *Comet* shedding card game
//! (https://qlayground.net/games/view.php?slug=comet), played against a
//! simple AI opponent.
//!
//! The game rules themselves live in [`comet`]; this module is purely
//! responsible for wiring that engine up to the DOM with `wasm-bindgen`
//! and `web-sys`: rendering the human's hand as clickable cards, showing
//! the AI's hand size and the running log, and driving the AI's turn on
//! a short timer so its moves are easy to follow.

mod comet;

use std::cell::RefCell;
use std::rc::Rc;

use comet::{Card, Game, Player};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn document() -> Document {
    window().document().expect("no `document` on `window`")
}

/// Random source for shuffling, backed by `Math.random()`.
fn js_rand01() -> f64 {
    js_sys::Math::random()
}

/// Shared handle to the game plus the DOM root it renders into.
struct App {
    game: Game,
    root: Element,
}

type Shared = Rc<RefCell<App>>;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook();

    let root = document()
        .get_element_by_id("comet-app")
        .expect("no element with id `comet-app`");

    let app: Shared = Rc::new(RefCell::new(App { game: Game::new(js_rand01), root }));
    render(&app);
    maybe_schedule_ai_turn(&app);
    Ok(())
}

fn build_card_button(
    doc: &Document,
    app: &Shared,
    index: usize,
    card: Card,
    playable: bool,
) -> Result<Element, JsValue> {
    let button = doc.create_element("button")?;
    button.set_class_name(if playable { "card playable" } else { "card" });
    button.set_text_content(Some(&card.display()));

    let button_el: &web_sys::HtmlButtonElement = button.dyn_ref().expect("is a button");
    button_el.set_disabled(!playable);

    if playable {
        let app = app.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            let mut needs_ai_turn = false;
            {
                let mut state = app.borrow_mut();
                match state.game.play(Player::Human, index) {
                    Ok(()) => needs_ai_turn = true,
                    Err(err) => state.game.log.push(err),
                }
            }
            render(&app);
            if needs_ai_turn {
                maybe_schedule_ai_turn(&app);
            }
        });
        button_el.set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    Ok(button)
}

fn render(app: &Shared) {
    let doc = document();
    let root = app.borrow().root.clone();
    root.set_inner_html("");

    let state = app.borrow();
    let status = doc.create_element("p").unwrap();
    status.set_class_name("status");
    status.set_text_content(Some(&status_text(&state.game)));
    root.append_child(&status).unwrap();

    let ai_info = doc.create_element("p").unwrap();
    ai_info.set_class_name("ai-info");
    ai_info.set_text_content(Some(&format!("AI hand: {} card(s)", state.game.ai_hand.len())));
    root.append_child(&ai_info).unwrap();

    let hand_container = doc.create_element("div").unwrap();
    hand_container.set_class_name("hand");
    let is_human_turn = state.game.winner.is_none() && state.game.turn == Player::Human;
    let valid = state.game.valid_play_indices(Player::Human);
    let human_hand = state.game.human_hand.clone();
    drop(state);

    for (index, card) in human_hand.into_iter().enumerate() {
        let playable = is_human_turn && valid.contains(&index);
        let button = build_card_button(&doc, app, index, card, playable).unwrap();
        hand_container.append_child(&button).unwrap();
    }
    root.append_child(&hand_container).unwrap();

    if is_human_turn && valid.is_empty() {
        let hint = doc.create_element("p").unwrap();
        hint.set_class_name("hint");
        hint.set_text_content(Some("No legal play — the AI will get a turn instead."));
        root.append_child(&hint).unwrap();
    }

    let log_heading = doc.create_element("h3").unwrap();
    log_heading.set_text_content(Some("Log"));
    root.append_child(&log_heading).unwrap();

    let log_list = doc.create_element("ul").unwrap();
    log_list.set_class_name("log");
    let state = app.borrow();
    for entry in state.game.log.iter().rev().take(8) {
        let item = doc.create_element("li").unwrap();
        item.set_text_content(Some(entry));
        log_list.append_child(&item).unwrap();
    }
    drop(state);
    root.append_child(&log_list).unwrap();

    let new_game = doc.create_element("button").unwrap();
    new_game.set_class_name("new-game");
    new_game.set_text_content(Some("New Game"));
    {
        let app = app.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            {
                let mut state = app.borrow_mut();
                state.game = Game::new(js_rand01);
            }
            render(&app);
            maybe_schedule_ai_turn(&app);
        });
        new_game
            .dyn_ref::<web_sys::HtmlButtonElement>()
            .unwrap()
            .set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }
    root.append_child(&new_game).unwrap();
}

fn status_text(game: &Game) -> String {
    if let Some(outcome) = &game.winner {
        return format!(
            "{} won the game{}!",
            outcome.winner.label(),
            if outcome.won_with_comet { " with the Comet — bonus win" } else { "" }
        );
    }
    let turn = if game.turn == Player::Human { "Your turn" } else { "AI's turn" };
    let requirement = match game.required_above {
        Some(min) => format!("play a card higher than {} (or the Comet)", min),
        None => "start a new run with any card".to_string(),
    };
    format!("{turn} — {requirement}.")
}

/// If it's the AI's turn, plays its move after a brief delay so the move
/// is visible instead of instantaneous.
fn maybe_schedule_ai_turn(app: &Shared) {
    let is_ai_turn = {
        let state = app.borrow();
        state.game.winner.is_none() && state.game.turn == Player::Ai
    };
    if !is_ai_turn {
        return;
    }

    let app = app.clone();
    let closure = Closure::<dyn FnMut()>::new(move || {
        let mut needs_ai_turn = false;
        {
            let mut state = app.borrow_mut();
            match state.game.ai_choose_play() {
                Some(index) => {
                    if state.game.play(Player::Ai, index).is_ok() {
                        needs_ai_turn = state.game.winner.is_none() && state.game.turn == Player::Ai;
                    }
                }
                None => state.game.log.push("The AI has no valid play and passes.".to_string()),
            }
        }
        render(&app);
        if needs_ai_turn {
            maybe_schedule_ai_turn(&app);
        }
    });
    window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            600,
        )
        .expect("failed to schedule AI turn");
    closure.forget();
}

/// Installs a panic hook that forwards Rust panics to the browser console,
/// which is invaluable for debugging wasm without a native backtrace.
fn console_error_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("{info}").into());
    }));
}
