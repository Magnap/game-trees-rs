pub mod nim;
pub mod backgammon;

use std::collections::HashMap;
use std::hash::Hash;
#[cfg(feature = "debug")]
use std::fmt::Debug;

pub type Score = f64;
pub type ScoreBoard<G: GameState> = HashMap<G::Player, Score>;

pub trait GameState: Eq + Hash + Clone + Sync + Send {
    #[cfg(feature = "debug")]
    type Move: Eq + Hash + Clone + Sync + Send + Debug;
    #[cfg(not(feature = "debug"))]
    type Move: Eq + Hash + Clone + Sync + Send;

    #[cfg(feature = "debug")]
    type Player: Eq + Hash + Clone + Sync + Send + Debug;
    #[cfg(not(feature = "debug"))]
    type Player: Eq + Hash + Clone + Sync + Send;

    fn new() -> Self;
    fn apply(&mut self, Self::Move);
    fn legal_moves(&self) -> Vec<Self::Move>;
    fn players() -> Vec<Self::Player>;
    fn current_player(&self) -> Self::Player;
    fn scores(&self) -> Option<ScoreBoard<Self>>;
    fn finished(&self) -> bool;
    fn possible_moves(&self) -> Vec<Self::Move> {
        if self.finished() {
            Vec::new()
        } else {
            self.legal_moves()
        }
    }
}
