pub mod nim;
pub mod backgammon;

use std::collections::HashMap;
use std::hash::Hash;
#[cfg(feature = "debug")]
use std::fmt::Debug;

pub type Score = f64;
pub type ScoreBoard<G: Game> = HashMap<G::Player, Score>;

pub trait Game {
    #[cfg(feature = "debug")]
    type Move: Eq + Hash + Clone + Sync + Send + Debug;
    #[cfg(not(feature = "debug"))]
    type Move: Eq + Hash + Clone + Sync + Send;

    #[cfg(feature = "debug")]
    type State: Eq + Hash + Clone + Sync + Send + Debug;
    #[cfg(not(feature = "debug"))]
    type State: Eq + Hash + Clone + Sync + Send;

    #[cfg(feature = "debug")]
    type Player: Eq + Hash + Clone + Sync + Send + Debug;
    #[cfg(not(feature = "debug"))]
    type Player: Eq + Hash + Clone + Sync + Send;

    fn new() -> Self::State;
    fn apply(&mut Self::State, Self::Move);
    fn legal_moves(&Self::State) -> Vec<Self::Move>;
    fn players() -> Vec<Self::Player>;
    fn current_player(&Self::State) -> Self::Player;
    fn points(&Self::State) -> Option<ScoreBoard<Self>>;
    fn finished(&Self::State) -> bool;
}

pub fn possible_moves<G: Game>(s: &G::State) -> Vec<G::Move> {
    if G::finished(s) {
        Vec::new()
    } else {
        G::legal_moves(s)
    }
}
