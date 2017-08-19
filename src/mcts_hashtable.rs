extern crate rand;
extern crate fnv;

use game;
use game::{Game, Score, ScoreBoard};
use std::collections::HashMap;
use self::fnv::FnvHashMap;
use self::rand::{thread_rng, Rng};
use std::f64;
use std::cmp::Ordering;

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Meta<G: Game> {
    pub scoreboard: ScoreBoard<G>,
    pub playouts: u32,
    pub moves: FnvHashMap<G::Move, G::State>,
}

impl<G: Game> Meta<G> {
    fn new() -> Self {
        Self::with_state(G::new())
    }

    fn with_state(s: G::State) -> Self {
        let ms = game::possible_moves::<G>(&s);
        let mut table = HashMap::default();
        for m in ms.into_iter() {
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
pub struct MctsTable<G: Game>(pub FnvHashMap<G::State, Meta<G>>);

impl<G: Game> MctsTable<G> {
    pub fn new() -> Self {
        Self::with_state(G::new())
    }

    pub fn with_state(s: G::State) -> Self {
        let mut table = MctsTable(HashMap::default());
        table.insert(s);
        table
    }

    fn insert(&mut self, s: G::State) {
        self.0.insert(s.clone(), Meta::with_state(s));
    }

    pub fn best_choice(&self, s: &G::State) -> Option<&G::Move> {
        self.0.get(s).and_then(|meta| {
            let moves = meta.moves.iter().map(|(m, new)| {
                let weight = match self.0.get(new) {
                    None => f64::INFINITY,
                    Some(v) => {
                        if v.playouts == 0 {
                            f64::INFINITY
                        } else {
                            v.scoreboard[&G::current_player(s)] / (v.playouts as f64) +
                                f64::sqrt(2.0 * (meta.playouts as f64).ln() / (v.playouts as f64))
                        }
                    }
                };
                (m, weight)
            });
            let bests = moves
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
            thread_rng().choose(&bests).map(|m| *m)
        })
    }

    pub fn playout(&mut self, s: &G::State) {
        self.playout_(s);
    }

    fn playout_(&mut self, s: &G::State) -> ScoreBoard<G> {
        if self.0.get(&s).is_none() {
            self.insert(s.clone());
        }
        if self.0.get(&s).is_some() {
            let scores: ScoreBoard<G>;
            match self.best_choice(&s).map(|m| m.clone()) {
                Some(best_move) => {
                    let mut new = s.clone();
                    G::apply(&mut new, best_move);
                    scores = self.playout_(&new);
                }
                None => {
                    scores = G::points(&s).unwrap_or_else(|| all_scores_zero::<G>());
                }
            }
            let v = self.0.get_mut(s).unwrap();
            for (key, score) in v.scoreboard.iter_mut() {
                *score += scores[key]
            }
            v.playouts += 1;
            scores
        } else {
            panic!("Node should have been initialized by now");
        }
    }
}
