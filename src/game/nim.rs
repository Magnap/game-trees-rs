use game::{GameState, Score};
use std::collections::HashMap;
use std::cmp::min;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Nim(u32, bool);

impl GameState for Nim {
    type Move = u32;
    type Player = bool;

    fn new() -> Self {
        Nim(0, false)
    }

    fn apply(&mut self, m: Self::Move) {
        self.0 += m;
        self.1 = !self.1;
    }

    fn players() -> Vec<Self::Player> {
        vec![false, true]
    }

    fn current_player(&self) -> Self::Player {
        self.1
    }

    fn legal_moves(&self) -> Vec<Self::Move> {
        (1..(min(10, 100 - self.0) + 1)).collect()
    }

    fn scores(&self) -> Option<HashMap<Self::Player, Score>> {
        if self.finished() {
            let mut m = HashMap::new();
            let p = self.1;
            m.insert(p, if self.0 == 100 { -1.0 } else { -5.0 });
            m.insert(!p, if self.0 == 100 { 1.0 } else { -5.0 });
            Some(m)
        } else {
            None
        }
    }

    fn finished(&self) -> bool {
        self.0 >= 100
    }
}
