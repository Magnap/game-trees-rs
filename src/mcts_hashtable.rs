extern crate rand;
extern crate chashmap;

use game;
use game::{Game, ScoreBoard};
use self::chashmap::CHashMap;
use std::collections::HashMap;
use self::rand::{thread_rng, Rng};
use std::f64;
use std::cmp::Ordering;
use std::mem;

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Meta<G: Game> {
    pub scoreboard: ScoreBoard<G>,
    pub playouts: u32,
    pub moves: HashMap<G::Move, G::State>,
}

impl<G: Game> Meta<G> {
    fn with_state(s: G::State) -> Self {
        let ms = game::possible_moves::<G>(&s);
        let mut table = HashMap::default();
        for m in ms {
            let mut new = s.clone();
            G::apply(&mut new, m.clone());
            table.insert(m, new);
        }
        Meta {
            scoreboard: all_scores_zero::<G>(),
            playouts: 0,
            moves: table,
        }
    }
}

fn all_scores_zero<G: Game>() -> ScoreBoard<G> {
    G::players().into_iter().map(|p| (p, 0.0)).collect()
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct MctsTable<G: Game>(pub CHashMap<G::State, Meta<G>>);

impl<G: Game> Default for MctsTable<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: Game> MctsTable<G> {
    pub fn new() -> Self {
        Self::with_state(G::new())
    }

    pub fn with_state(s: G::State) -> Self {
        let table = MctsTable(CHashMap::default());
        table.insert(s);
        table
    }

    fn insert(&self, s: G::State) {
        self.0.insert(s.clone(), Meta::with_state(s));
    }

    pub fn best_choice(&self, s: &G::State) -> Option<G::Move> {
        let lock = self.0.get(s);
        let moves: Vec<_> = lock.map(|meta| {
            meta.moves
                .iter()
                .map(|(m, new)| {
                    let weight = match self.0.get(new) {
                        None => f64::INFINITY,
                        Some(v) => {
                            if v.playouts == 0 {
                                f64::INFINITY
                            } else {
                                v.scoreboard[&G::current_player(s)] / (v.playouts as f64) +
                                    f64::sqrt(
                                        2.0 * (meta.playouts as f64).ln() / (v.playouts as f64),
                                    )
                            }
                        }
                    };
                    (m.clone(), weight)
                })
                .collect()
        }).unwrap_or_else(Vec::new);
        let bests = moves
            .into_iter()
            .fold((Vec::new(), f64::NEG_INFINITY), |(mut ms, mut best),
             (m, weight)| {
                match weight.partial_cmp(&best) {
                    Some(Ordering::Equal) => ms.push(m),
                    Some(Ordering::Greater) => {
                        best = weight;
                        ms = vec![m];
                    }
                    _ => {}
                }
                (ms, best)
            })
            .0;
        thread_rng().choose(&bests).map(|m| (*m).clone())
    }

    pub fn playout(&self, s: &G::State) {
        self.playout_(s);
    }

    fn playout_(&self, s: &G::State) -> ScoreBoard<G> {
        let lock = self.0.get(s);
        let x = lock.is_none();
        mem::drop(lock);
        if x {
            self.insert(s.clone());
        }
        let lock: Option<chashmap::ReadGuard<_, _>> = self.0.get(s);
        let x = lock.is_some();
        mem::drop(lock);
        if x {
            let scores: ScoreBoard<G>;
            let best_move_opt = self.best_choice(s);
            match best_move_opt {
                Some(best_move) => {
                    let mut new = s.clone();
                    G::apply(&mut new, best_move);
                    scores = self.playout_(&new);
                }
                None => {
                    scores = G::points(s).unwrap_or_else(all_scores_zero::<G>);
                }
            }
            {
                let mut v = self.0.get_mut(s).unwrap();
                v.playouts += 1;
                for (key, score) in &mut v.scoreboard {
                    *score += scores[key]
                }
            }
            scores
        } else {
            panic!("Node should have been initialized by now");
        }
    }
}
