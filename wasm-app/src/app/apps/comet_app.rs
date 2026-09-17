//! The Comet card game, ported to run as an [`App`](crate::desktop::App)
//! inside a desktop window. The rules engine ([`crate::comet`]) is
//! untouched; this module only draws it with the quad/text [`Painter`]
//! and turns pointer clicks into [`Game::play`] calls.

use crate::comet::{Card, Game, Player, Suit};
use crate::app::desktop::App;
use crate::app::geometry::Rect;
use crate::app::painter::{Color, Painter};

const TEXT: Color = Color::rgb(0xe6, 0xed, 0xf3);
const DIM_TEXT: Color = Color::rgb(0x8b, 0x94, 0x9c);
const CARD_BG: Color = Color::rgb(0x21, 0x26, 0x2d);
const CARD_PLAYABLE_BG: Color = Color::rgb(0x1f, 0x6f, 0xeb);
const CARD_BORDER: Color = Color::rgb(0x30, 0x36, 0x3d);
const BUTTON_BG: Color = Color::rgb(0x30, 0x36, 0x3d);

const GLYPH_SCALE: f32 = 2.0;
const CARD_W: f32 = 56.0;
const CARD_H: f32 = 32.0;
const CARD_GAP: f32 = 6.0;

fn rank_label(rank: u8) -> String {
    match rank {
        11 => "J".to_string(),
        12 => "Q".to_string(),
        13 => "K".to_string(),
        14 => "A".to_string(),
        n => n.to_string(),
    }
}

fn suit_letter(suit: Suit) -> &'static str {
    match suit {
        Suit::Clubs => "C",
        Suit::Diamonds => "D",
        Suit::Hearts => "H",
        Suit::Spades => "S",
    }
}

fn card_label(card: Card) -> String {
    match card {
        Card::Comet => "COMET".to_string(),
        Card::Normal { rank, suit } => format!("{}{}", rank_label(rank), suit_letter(suit)),
    }
}

fn js_rand01() -> f64 {
    js_sys::Math::random()
}

pub struct CometApp {
    game: Game,
    card_rects: Vec<Rect>,
    new_game_rect: Rect,
    ai_move_at: Option<f64>,
}

impl CometApp {
    pub fn new() -> Self {
        Self {
            game: Game::new(js_rand01),
            card_rects: Vec::new(),
            new_game_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            ai_move_at: None,
        }
    }

    fn status_text(&self) -> String {
        if let Some(outcome) = &self.game.winner {
            return format!(
                "{} WON THE GAME{}!",
                outcome.winner.label(),
                if outcome.won_with_comet { " WITH THE COMET - BONUS WIN" } else { "" }
            );
        }
        let turn = if self.game.turn == Player::Human { "YOUR TURN" } else { "AI'S TURN" };
        let requirement = match self.game.required_above {
            Some(min) => format!("PLAY HIGHER THAN {} (OR COMET)", rank_label(min)),
            None => "START A NEW RUN WITH ANY CARD".to_string(),
        };
        format!("{turn} - {requirement}")
    }
}

impl Default for CometApp {
    fn default() -> Self {
        Self::new()
    }
}

impl App for CometApp {
    fn title(&self) -> &str {
        "COMET"
    }

    fn update(&mut self, now_ms: f64) {
        let is_ai_turn = self.game.winner.is_none() && self.game.turn == Player::Ai;
        if !is_ai_turn {
            self.ai_move_at = None;
            return;
        }
        match self.ai_move_at {
            None => self.ai_move_at = Some(now_ms + 600.0),
            Some(at) if now_ms >= at => {
                self.ai_move_at = None;
                match self.game.ai_choose_play() {
                    Some(index) => {
                        let _ = self.game.play(Player::Ai, index);
                    }
                    None => self.game.log.push("The AI has no valid play and passes.".to_string()),
                }
            }
            Some(_) => {}
        }
    }

    fn render(&mut self, painter: &mut Painter, content: Rect) {
        let mut y = content.y + 10.0;
        painter.text(content.x + 10.0, y, GLYPH_SCALE, TEXT, &self.status_text());
        y += Painter::text_height(GLYPH_SCALE) + 10.0;

        painter.text(
            content.x + 10.0,
            y,
            GLYPH_SCALE,
            DIM_TEXT,
            &format!("AI HAND: {} CARDS", self.game.ai_hand.len()),
        );
        y += Painter::text_height(GLYPH_SCALE) + 14.0;

        let is_human_turn = self.game.winner.is_none() && self.game.turn == Player::Human;
        let valid = self.game.valid_play_indices(Player::Human);

        self.card_rects.clear();
        let mut x = content.x + 10.0;
        let row_start_y = y;
        for (index, card) in self.game.human_hand.iter().enumerate() {
            if x + CARD_W > content.x + content.w - 10.0 {
                x = content.x + 10.0;
                y += CARD_H + CARD_GAP;
            }
            let rect = Rect::new(x, y, CARD_W, CARD_H);
            let playable = is_human_turn && valid.contains(&index);
            painter.rect(rect, if playable { CARD_PLAYABLE_BG } else { CARD_BG });
            painter.rect_outline(rect, 1.0, CARD_BORDER);
            let label = card_label(*card);
            let text_w = Painter::text_width(&label, GLYPH_SCALE * 0.7);
            painter.text(
                rect.x + (rect.w - text_w) / 2.0,
                rect.y + (rect.h - Painter::text_height(GLYPH_SCALE * 0.7)) / 2.0,
                GLYPH_SCALE * 0.7,
                if playable { TEXT } else { DIM_TEXT },
                &label,
            );
            self.card_rects.push(rect);
            x += CARD_W + CARD_GAP;
        }
        y = y.max(row_start_y) + CARD_H + 16.0;

        painter.text(content.x + 10.0, y, GLYPH_SCALE, DIM_TEXT, "LOG");
        y += Painter::text_height(GLYPH_SCALE) + 6.0;
        let log_bottom = content.y + content.h - 40.0;
        for entry in self.game.log.iter().rev() {
            if y > log_bottom {
                break;
            }
            painter.text(content.x + 10.0, y, GLYPH_SCALE * 0.7, DIM_TEXT, entry);
            y += Painter::text_height(GLYPH_SCALE * 0.7) + 4.0;
        }

        let btn_w = Painter::text_width("NEW GAME", GLYPH_SCALE) + 20.0;
        let btn_h = Painter::text_height(GLYPH_SCALE) + 12.0;
        self.new_game_rect = Rect::new(
            content.x + (content.w - btn_w) / 2.0,
            content.y + content.h - btn_h - 8.0,
            btn_w,
            btn_h,
        );
        painter.rect(self.new_game_rect, BUTTON_BG);
        painter.rect_outline(self.new_game_rect, 1.0, CARD_BORDER);
        painter.text(
            self.new_game_rect.x + 10.0,
            self.new_game_rect.y + 6.0,
            GLYPH_SCALE,
            TEXT,
            "NEW GAME",
        );
    }

    fn on_pointer_down(&mut self, pos: (f32, f32)) {
        let (x, y) = pos;
        if self.new_game_rect.contains(x, y) {
            self.game = Game::new(js_rand01);
            self.ai_move_at = None;
            return;
        }

        let is_human_turn = self.game.winner.is_none() && self.game.turn == Player::Human;
        if !is_human_turn {
            return;
        }
        for (index, rect) in self.card_rects.iter().enumerate() {
            if rect.contains(x, y) {
                if let Err(err) = self.game.play(Player::Human, index) {
                    self.game.log.push(err);
                }
                return;
            }
        }
    }
}
