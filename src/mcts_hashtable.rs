extern crate rand;
extern crate fnv;

use game;
use game::{Game, ScoreBoard};
use self::fnv::FnvHashMap;
use self::rand::{thread_rng, Rng};
use std::f64;
use std::cmp::Ordering;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone)]
pub struct Meta<G: Game + Clone> {
    pub scoreboard: ScoreBoard<G>,
    pub playouts: u32,
    pub moves: FnvHashMap<G::Move, (G::State, usize)>,
    paths: usize,
}

impl<G: Game + Clone> Meta<G> {
    fn with_state(s: G::State) -> Self {
        let ms = game::possible_moves::<G>(&s);
        let mut table = FnvHashMap::default();
        for m in ms {
            let mut new = s.clone();
            G::apply(&mut new, m.clone());
            table.insert(m, (new, 0));
        }
        Meta {
            scoreboard: all_scores_zero::<G>(),
            playouts: 0,
            moves: table,
            paths: 0,
        }
    }
}

fn all_scores_zero<G: Game>() -> ScoreBoard<G> {
    G::players().into_iter().map(|p| (p, 0.0)).collect()
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct MctsTable<G: Game + Clone>(pub FnvHashMap<G::State, Meta<G>>);

impl<G: Game + Clone> Default for MctsTable<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: Game + Clone> MctsTable<G> {
    pub fn new() -> Self {
        Self::with_state(G::new())
    }

    pub fn with_state(s: G::State) -> Self {
        let mut table = MctsTable(FnvHashMap::default());
        table.insert(s);
        table
    }

    fn insert(&mut self, s: G::State) {
        self.0.insert(s.clone(), Meta::with_state(s));
    }

    pub fn best_choice(&self, s: &G::State) -> Option<G::Move> {
        self.0.get(s).and_then(|meta| {
            meta.moves
                .iter()
                .max_by_key(|&(_, new)| {
                    self.0
                        .get(&new.0)
                        .map(|new_meta| new_meta.playouts)
                        .unwrap_or(0)
                })
                .map(|(m, _)| m.clone())
        })
    }

    fn best_choice_(&self, s: &G::State) -> Option<G::Move> {
        let moves: Vec<_> = self.0
            .get(s)
            .map(|meta| {
                meta.moves
                    .iter()
                    .map(|(m, new)| {
                        let weight = match self.0.get(&new.0) {
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
            })
            .unwrap_or_else(Vec::new);
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

    pub fn playout(&mut self, s: &G::State, max_its: u32) {
        self.playout_(s, max_its);
    }

    fn playout_(&mut self, s: &G::State, max_its: u32) -> ScoreBoard<G> {
        if self.0.get(s).is_none() {
            self.insert(s.clone());
        }
        if self.0.get(s).is_some() {
            let scores: ScoreBoard<G>;
            let best_move_opt = if max_its > 0 {
                self.best_choice_(s)
            } else {
                None
            };
            match best_move_opt {
                Some(best_move) => {
                    let mut new = s.clone();
                    {
                        let v = self.0.get_mut(s).unwrap();
                        v.moves.get_mut(&best_move).unwrap().1 += 1;
                    }
                    G::apply(&mut new, best_move);
                    scores = self.playout_(&new, max_its - 1);
                }
                None => {
                    scores = G::points(s).unwrap_or_else(all_scores_zero::<G>);
                }
            }
            let mut v = self.0.get_mut(s).unwrap();
            v.paths += 1;
            v.playouts += 1;
            for (key, score) in &mut v.scoreboard {
                *score += scores[key]
            }
            scores
        } else {
            panic!("Node should have been initialized by now");
        }
    }

    pub fn garbage_collect(&mut self, s: &G::State) {
        let mut to_be_gced = vec![s.clone()];
        let mut initial = true;
        while !to_be_gced.is_empty() {
            let curr = to_be_gced.pop().unwrap();
            let exists = self.0.get(&curr).is_some();
            if exists {
                let old_meta: Meta<G>;
                {
                    old_meta = self.0[&curr].clone();
                }
                if old_meta.paths == 0 || initial {
                    self.0.remove(&curr);
                    for (_, (new, touches)) in old_meta.moves {
                        if touches > 0 {
                            self.0.get_mut(&new).map(|meta| meta.paths -= touches);
                            to_be_gced.push(new);
                        }
                    }
                }
            }
            initial = false;
        }
    }
}
