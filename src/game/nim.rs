use game::{Game, Score};
use std::collections::HashMap;
use std::cmp::min;

pub struct Nim;

impl Game for Nim {
    type Move = u32;
    type State = (u32, bool);
    type Player = bool;

    fn new() -> Self::State {
        (0, false)
    }

    fn apply(s: &mut Self::State, m: Self::Move) {
        s.0 += m;
        s.1 = !s.1;
    }

    fn players() -> Vec<Self::Player> {
        vec![false, true]
    }

    fn current_player(s: &Self::State) -> Self::Player {
        s.1
    }

    fn legal_moves(s: &Self::State) -> Vec<Self::Move> {
        (1..(min(10, 100 - s.0) + 1)).collect()
    }

    fn scores(s: &Self::State) -> Option<HashMap<Self::Player, Score>> {
        if Self::finished(s) {
            let mut m = HashMap::new();
            let p = s.1;
            m.insert(p, if s.0 == 100 { -1.0 } else { -5.0 });
            m.insert(!p, if s.0 == 100 { 1.0 } else { -5.0 });
            Some(m)
        } else {
            None
        }
    }

    fn finished(s: &Self::State) -> bool {
        s.0 >= 100
    }
}
