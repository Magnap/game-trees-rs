use std::collections::HashMap;
use game::Game;

#[derive(Debug)]
pub struct GameTree<G: Game, T> {
    pub value: T,
    pub children: HashMap<G::Move, GameTree<G, T>>,
}
