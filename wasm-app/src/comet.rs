//! Core game engine for a prototype of the *Comet* shedding card game
//! (https://qlayground.net/games/view.php?slug=comet).
//!
//! This is a simplified, self-contained implementation of the game's
//! defining mechanic: players take turns extending an ascending run of
//! cards (any suit, strictly increasing rank), a King ends the run and
//! lets its player start a fresh one, and the single wildcard "Comet"
//! card (traditionally the 9 of Diamonds) can be played at any point to
//! keep a run alive. The first player to empty their hand wins; doing so
//! by playing the Comet as the final card is a bonus win.
//!
//! No UI or `wasm-bindgen` types live here — this module only knows about
//! cards and rules, which keeps it easy to reason about and to reuse from
//! `lib.rs`.

pub const KING_RANK: u8 = 13;
pub const LOWEST_RANK: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    fn symbol(self) -> &'static str {
        match self {
            Suit::Clubs => "♣",
            Suit::Diamonds => "♦",
            Suit::Hearts => "♥",
            Suit::Spades => "♠",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Card {
    /// A regular card. `rank` runs from 2 (lowest) to 14 (Ace, highest),
    /// with 11/12/13 standing in for Jack/Queen/King.
    Normal { rank: u8, suit: Suit },
    /// The wildcard Comet card (physically the 9 of Diamonds). It can be
    /// played on top of any rank and simply raises the run's required
    /// rank by one.
    Comet,
}

impl Card {
    pub fn is_king(self) -> bool {
        matches!(self, Card::Normal { rank: KING_RANK, .. })
    }

    pub fn display(self) -> String {
        match self {
            Card::Comet => "☄ Comet".to_string(),
            Card::Normal { rank, suit } => format!("{}{}", rank_label(rank), suit.symbol()),
        }
    }

    /// Sort key: real cards first by rank, Comet sorted alongside 9s since
    /// that's the card it physically replaces.
    fn sort_key(self) -> (u8, u8) {
        match self {
            Card::Normal { rank, suit } => (rank, suit as u8),
            Card::Comet => (9, 4),
        }
    }
}

fn rank_label(rank: u8) -> String {
    match rank {
        11 => "J".to_string(),
        12 => "Q".to_string(),
        13 => "K".to_string(),
        14 => "A".to_string(),
        n => n.to_string(),
    }
}

/// Builds a standard 52-card deck (ranks 2..=14, four suits) with the
/// normal 9 of Diamonds swapped out for the wildcard Comet card.
pub fn build_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    for &suit in &[Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        for rank in LOWEST_RANK..=14 {
            if rank == 9 && suit == Suit::Diamonds {
                deck.push(Card::Comet);
            } else {
                deck.push(Card::Normal { rank, suit });
            }
        }
    }
    deck
}

/// Fisher-Yates shuffle driven by a caller-supplied random source so this
/// module stays free of any platform-specific RNG dependency.
pub fn shuffle(deck: &mut [Card], mut rand01: impl FnMut() -> f64) {
    for i in (1..deck.len()).rev() {
        let j = (rand01() * (i as f64 + 1.0)) as usize;
        let j = j.min(i);
        deck.swap(i, j);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Player {
    Human,
    Ai,
}

impl Player {
    pub fn other(self) -> Player {
        match self {
            Player::Human => Player::Ai,
            Player::Ai => Player::Human,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Player::Human => "You",
            Player::Ai => "The AI",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub winner: Player,
    pub won_with_comet: bool,
}

/// Full mutable state of a two-player Comet game.
pub struct Game {
    pub human_hand: Vec<Card>,
    pub ai_hand: Vec<Card>,
    /// Rank the next card must exceed, or `None` if a fresh run may be
    /// started with any card.
    pub required_above: Option<u8>,
    pub turn: Player,
    /// Player who last successfully played a card; leads the next run if
    /// the current one dies with both players passing.
    last_player: Option<Player>,
    pub winner: Option<Outcome>,
    pub log: Vec<String>,
}

impl Game {
    pub fn new(mut rand01: impl FnMut() -> f64) -> Self {
        let mut deck = build_deck();
        shuffle(&mut deck, &mut rand01);

        let mut human_hand: Vec<Card> = deck.drain(0..26).collect();
        let mut ai_hand: Vec<Card> = deck;

        human_hand.sort_by_key(|c| c.sort_key());
        ai_hand.sort_by_key(|c| c.sort_key());

        // Whoever holds the lowest card (2 of Clubs, if present) leads.
        let human_leads = human_hand
            .iter()
            .any(|c| matches!(c, Card::Normal { rank: LOWEST_RANK, .. }));

        let mut game = Game {
            human_hand,
            ai_hand,
            required_above: None,
            turn: if human_leads { Player::Human } else { Player::Ai },
            last_player: None,
            winner: None,
            log: vec!["New game dealt. 26 cards each; lowest card leads.".to_string()],
        };
        game.log.push(format!("{} leads the first run.", game.turn.label()));
        game
    }

    fn hand(&self, player: Player) -> &Vec<Card> {
        match player {
            Player::Human => &self.human_hand,
            Player::Ai => &self.ai_hand,
        }
    }

    fn hand_mut(&mut self, player: Player) -> &mut Vec<Card> {
        match player {
            Player::Human => &mut self.human_hand,
            Player::Ai => &mut self.ai_hand,
        }
    }

    pub fn is_valid_play(&self, card: Card) -> bool {
        match card {
            Card::Comet => true,
            Card::Normal { rank, .. } => self.required_above.is_none_or(|min| rank > min),
        }
    }

    pub fn valid_play_indices(&self, player: Player) -> Vec<usize> {
        self.hand(player)
            .iter()
            .enumerate()
            .filter(|(_, &c)| self.is_valid_play(c))
            .map(|(i, _)| i)
            .collect()
    }

    fn has_valid_play(&self, player: Player) -> bool {
        self.hand(player).iter().any(|&c| self.is_valid_play(c))
    }

    /// Plays card at `index` from `player`'s hand. Returns an error string
    /// if it isn't that player's turn or the card can't legally be played.
    pub fn play(&mut self, player: Player, index: usize) -> Result<(), String> {
        if self.winner.is_some() {
            return Err("The game is already over.".to_string());
        }
        if player != self.turn {
            return Err("It isn't your turn.".to_string());
        }
        let card = *self
            .hand(player)
            .get(index)
            .ok_or_else(|| "No such card.".to_string())?;
        if !self.is_valid_play(card) {
            return Err("That card can't be played on the current run.".to_string());
        }

        self.hand_mut(player).remove(index);
        self.log.push(format!("{} plays {}.", player.label(), card.display()));
        self.last_player = Some(player);

        let won_with_comet = card == Card::Comet;
        if self.hand(player).is_empty() {
            self.winner = Some(Outcome { winner: player, won_with_comet });
            self.log.push(if won_with_comet {
                format!("{} wins with the Comet as the final card! ✨ Bonus win!", player.label())
            } else {
                format!("{} has emptied their hand and wins!", player.label())
            });
            return Ok(());
        }

        if card.is_king() {
            self.required_above = None;
            self.log
                .push(format!("The run ends on the King. {} starts a new run.", player.label()));
        } else {
            let next_min = match card {
                Card::Comet => self.required_above.map(|m| m + 1).unwrap_or(LOWEST_RANK),
                Card::Normal { rank, .. } => rank,
            };
            self.required_above = Some(next_min);
            self.turn = player.other();
        }

        self.resolve_turn();
        Ok(())
    }

    /// Advances `self.turn` past any players who cannot play, restarting
    /// the run (led by whoever played last) if neither player can
    /// continue it.
    fn resolve_turn(&mut self) {
        if self.winner.is_some() {
            return;
        }
        let mut passes = 0;
        loop {
            if self.hand(self.turn).is_empty() {
                // Shouldn't normally happen (win is detected in `play`),
                // but guards against getting stuck.
                self.winner = Some(Outcome { winner: self.turn, won_with_comet: false });
                return;
            }
            if self.has_valid_play(self.turn) {
                return;
            }
            self.log.push(format!("{} has no valid play and passes.", self.turn.label()));
            passes += 1;
            self.turn = self.turn.other();
            if passes >= 2 {
                self.required_above = None;
                passes = 0;
                self.turn = self.last_player.unwrap_or(self.turn);
                self.log.push(format!(
                    "Neither player can continue. {} starts a new run.",
                    self.turn.label()
                ));
            }
        }
    }

    /// Simple AI: play the lowest-ranked legal card, keeping the Comet in
    /// reserve unless it's the only legal move (or it lets the AI win).
    pub fn ai_choose_play(&self) -> Option<usize> {
        let hand = self.hand(Player::Ai);
        let valid = self.valid_play_indices(Player::Ai);
        if valid.is_empty() {
            return None;
        }
        if hand.len() == 1 {
            return Some(valid[0]);
        }
        valid
            .into_iter()
            .min_by_key(|&i| match hand[i] {
                Card::Normal { rank, .. } => rank,
                // Rank the Comet just above an Ace so it's only chosen
                // when nothing else is playable.
                Card::Comet => 15,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_rand(seed: u64) -> impl FnMut() -> f64 {
        let mut state = seed;
        move || {
            // xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as f64 / u64::MAX as f64).abs()
        }
    }

    #[test]
    fn deck_has_52_unique_cards_with_one_comet() {
        let deck = build_deck();
        assert_eq!(deck.len(), 52);
        assert_eq!(deck.iter().filter(|c| matches!(c, Card::Comet)).count(), 1);
    }

    #[test]
    fn deal_splits_deck_evenly() {
        let game = Game::new(seeded_rand(42));
        assert_eq!(game.human_hand.len(), 26);
        assert_eq!(game.ai_hand.len(), 26);
    }

    #[test]
    fn a_full_game_terminates_with_a_winner() {
        let mut game = Game::new(seeded_rand(7));
        let mut guard = 0;
        while game.winner.is_none() {
            guard += 1;
            assert!(guard < 5000, "game did not terminate");
            let player = game.turn;
            let valid = game.valid_play_indices(player);
            assert!(!valid.is_empty(), "current player must always have a valid play");
            // Always play the first legal card - deterministic and simple.
            game.play(player, valid[0]).unwrap();
        }
        assert!(game.human_hand.is_empty() || game.ai_hand.is_empty());
    }

    #[test]
    fn playing_out_of_turn_is_rejected() {
        let mut game = Game::new(seeded_rand(1));
        let other = game.turn.other();
        assert!(game.play(other, 0).is_err());
    }

    #[test]
    fn king_ends_run_and_same_player_leads_again() {
        let mut game = Game::new(seeded_rand(1));
        let player = game.turn;
        let hand = vec![
            Card::Normal { rank: KING_RANK, suit: Suit::Clubs },
            Card::Normal { rank: 3, suit: Suit::Hearts },
        ];
        match player {
            Player::Human => game.human_hand = hand,
            Player::Ai => game.ai_hand = hand,
        }
        game.required_above = None;

        game.play(player, 0).unwrap();
        assert!(game.winner.is_none());
        assert_eq!(game.required_above, None, "King should reset the run");
        assert_eq!(game.turn, player, "the King's player leads the next run");
    }
}
