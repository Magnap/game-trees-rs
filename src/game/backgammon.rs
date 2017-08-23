use game::{Game, Score};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone)]
pub struct Backgammon;

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct Point(pub u8);

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct StackHeight(pub u8);

impl fmt::Display for StackHeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// None stands for the dice
pub type Player = Option<bool>;

pub type Roll = (u8, u8);

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum Location {
    Board(Point),
    Bar,
    Home,
}
use self::Location::*;

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Board(Point(n)) => write!(f, "{}", n),
            &Bar => write!(f, "bar"),
            &Home => write!(f, "home"),
        }
    }
}

// This needs type ascription, is there a better way to index a `Vec` by location?
// Ideally, this would depend on the player, making locations relative
impl From<Location> for usize {
    fn from(x: Location) -> usize {
        match x {
            Board(Point(n)) => n as usize,
            Bar => 0,
            Home => 25,
        }
    }
}

// This would be (Player, StackHeight)
// since there can only be one player's pieces on the points on the board
// but home and bar are exceptions to that
type Count = (StackHeight, StackHeight);

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct State {
    pub player: bool,
    roll_turn: bool,
    pub counts: Vec<Count>,
    pub dice: Roll,
}

// Location, amount to move by
pub type SingleMove = (Location, u8);

impl Game for Backgammon {
    // Result is used to make a sum type
    // deliberately "privileging" the player turns over the dice turns
    type Move = Result<Vec<SingleMove>, Roll>;
    type State = State;
    type Player = Player;

    fn new() -> Self::State {
        let mut v = Vec::new();
        for n in 0..26 {
            // This would be more elegant if locations were relative
            let white = match n {
                1 => 2,
                12 => 5,
                17 => 3,
                19 => 5,
                _ => 0,
            };
            let black = match n {
                6 => 5,
                8 => 3,
                13 => 5,
                24 => 2,
                _ => 0,
            };
            v.push((StackHeight(white), StackHeight(black)))
        }
        State {
            player: true,
            roll_turn: true,
            counts: v,
            dice: (0, 0),
        }
    }

    fn apply(s: &mut Self::State, m: Self::Move) {
        match m {
            Ok(v) => {
                // Assumes causally consistent ordering
                // otherwise points may underflow
                // (if an empty point is moved from, then moved to)
                for (l, n) in v {
                    // Non-lexical lifetimes would be nice here
                    {
                        let from = &mut s.counts[l.into(): usize];
                        if s.player {
                            (from.0).0 -= 1
                        } else {
                            (from.1).0 -= 1
                        };
                    }
                    let to_index = if s.player {
                        (if l == Bar { 0 } else { l.into(): usize }) as i8 + n as i8
                    } else {
                        (if l == Bar { 25 } else { l.into(): usize }) as i8 - n as i8
                    };
                    let home_p = if s.player {
                        to_index > 24
                    } else {
                        to_index < 1
                    };
                    let to_pos: Location;
                    if home_p {
                        to_pos = Home;
                    } else {
                        to_pos = Board(Point(to_index as u8))
                    }
                    let enemy_hit = if home_p {
                        false
                    } else {
                        any_loc(to_pos, !s.player, s)
                    };
                    // Another non-lexical lifetime
                    {
                        let mut to = &mut s.counts[to_pos.into(): usize];
                        if s.player {
                            (to.0).0 += 1
                        } else {
                            (to.1).0 += 1
                        };
                        if enemy_hit {
                            if s.player {
                                (to.1).0 -= 1;
                            } else {
                                (to.0).0 -= 1;
                            };
                        }
                    }
                    if enemy_hit {
                        let mut bar = &mut s.counts[Bar.into(): usize];
                        if s.player {
                            (bar.1).0 += 1;
                        } else {
                            (bar.0).0 += 1;
                        };
                    }
                }
                s.roll_turn = true;
                s.player = !s.player;
            }
            Err(pair) => {
                s.roll_turn = false;
                s.dice = pair;
            }
        }
    }

    fn players() -> Vec<Self::Player> {
        vec![None, Some(false), Some(true)]
    }

    fn current_player(s: &Self::State) -> Self::Player {
        if s.roll_turn { None } else { Some(s.player) }
    }

    fn legal_moves(s: &Self::State) -> Vec<Self::Move> {
        let mut v = Vec::new();
        if s.roll_turn {
            // Canonical representation of dice rolls:
            // highest die first
            for x in 1..7 {
                for y in 1..(x + 1) {
                    v.push(Err((x, y)))
                }
            }
        } else {
            let (x, y) = s.dice;
            let mut mss: Vec<Vec<_>> = Vec::new();
            mss.push(Vec::new());
            if x == y {
                mss.extend(legal_sequences(s, &[x, x, x, x]).into_iter());
            } else {
                mss.extend(legal_sequences(s, &[x, y]).into_iter());
                mss.extend(legal_sequences(s, &[y, x]).into_iter());
            }
            let max_moves = mss.iter()
                .map(|ms| ms.iter().map(|&(_, n)| n).sum::<u8>())
                .max()
                .unwrap_or(0);
            mss.retain(|ms| max_moves == ms.iter().map(|&(_, n)| n).sum::<u8>());
            for mut ms in mss {
                // Canonical representation of moves
                // Lowest position (counting as white) first
                // NOTE this breaks the causual consistency assumption of `apply`
                // however, underflow is rectified later,
                // so thanks to release mode having defined semantics for underflow
                // applying an entire move sequence preserves consistency
                ms.sort();
                v.push(Ok(ms))
            }
            v.sort();
            v.dedup();
        }
        v
    }

    fn scores(s: &Self::State) -> Option<HashMap<Self::Player, Score>> {
        if Self::finished(s) {
            let mut m = HashMap::new();
            // The person who just finished their turn is the one who has won
            let p = !s.player;
            let multiplier = match (all_homeboard(!p, s), any_loc(Home, !p, s)) {
                // Backgammon
                (false, false) => 3.0,
                // Gammon
                (true, false) => 2.0,
                // DISCUSS panic if inconsistent result?
                // this can only be (true, true)
                _ => 1.0,
            };
            m.insert(Some(p), multiplier);
            m.insert(Some(!p), -multiplier);
            // Dice always scoring 0 leads to them always considering all moves equally good as a player
            m.insert(None, 0.0);
            return Some(m);
        } else {
            None
        }
    }

    fn finished(s: &Self::State) -> bool {
        all_loc(Home, false, s) || all_loc(Home, true, s)
    }
}

fn legal_sequences(s: &State, dice: &[u8]) -> Vec<Vec<SingleMove>> {
    // Clone slice into Vec
    let mut new_dice = Vec::new();
    new_dice.extend(dice);
    let mut sequences = Vec::new();
    match new_dice.pop() {
        Some(roll) => {
            let mut positions = board();
            positions.push(Bar);
            positions.retain(|l| any_loc(*l, s.player, s));
            for p in positions {
                let mut s = s.clone();
                let m = (p, roll);
                if legal_move(&m, &s) {
                    <Backgammon as Game>::apply(&mut s, Ok(vec![m]));
                    s.player = !s.player;
                    // Recursion here is limited to depth 4
                    // and way simpler than doing backtracking
                    for mut ms in legal_sequences(&s, &new_dice) {
                        ms.push(m);
                        sequences.push(ms);
                    }
                    sequences.push(vec![m]);
                }
            }
        }
        None => {}
    }
    sequences
}

// This duplicates some of the logic in `apply`
// but is hopefully cheaper and simpler than having a `legal_state` function
fn legal_move(m: &SingleMove, s: &State) -> bool {
    let &(l, n) = m;
    let from_count = s.counts[l.into(): usize];
    let can_move_from = if s.player {
        from_count.0 >= StackHeight(1)
    } else {
        from_count.1 >= StackHeight(1)
    };
    let all_h_w = all_homeboard(true, &s);
    let all_h_b = all_homeboard(false, &s);
    let to_index = if s.player {
        (if l == Bar { 0 } else { l.into(): usize }) as i8 + n as i8
    } else {
        (if l == Bar { 25 } else { l.into(): usize }) as i8 - n as i8
    };
    let empty = (StackHeight(0), StackHeight(0));
    let enemy_count = s.counts.get(to_index as usize).unwrap_or(&empty);
    let can_move_to = if s.player {
        (to_index > 24 && all_h_w) || (to_index <= 24 && enemy_count.1 <= StackHeight(1))
    } else {
        (to_index < 1 && all_h_b) || (to_index >= 1 && enemy_count.0 <= StackHeight(1))
    };
    let bar_check = if any_loc(Bar, s.player, &s) {
        l == Bar
    } else {
        true
    };
    can_move_from && can_move_to && bar_check
}

// DISCUSS more correct and less wasteful but less convenient to return boxed slice
// DISCUSS should bar and home be included?
pub fn board() -> Vec<Location> {
    let mut v = Vec::with_capacity(24);
    for n in 1..25 {
        let n = Board(Point(n));
        v.push(n);
    }
    v
}

// DISCUSS return boxed slice?
// DISCUSS should home be included?
pub fn homeboard(p: bool) -> Vec<Location> {
    let mut v = Vec::with_capacity(6);
    for n in 1..7 {
        let n = Board(Point(n));
        let n = if p { flip(n) } else { n };
        v.push(n);
    }
    v.push(Home);
    v
}

fn all_homeboard(p: bool, s: &State) -> bool {
    homeboard(p)
        .into_iter()
        .map(|n| {
            let counts = s.counts[n.into(): usize];
            if p { (counts.0).0 } else { (counts.1).0 }
        })
        .sum::<u8>() == 15
}

fn any_loc(l: Location, p: bool, s: &State) -> bool {
    let counts = s.counts[l.into(): usize];
    StackHeight(0) < if p { counts.0 } else { counts.1 }
}

fn all_loc(l: Location, p: bool, s: &State) -> bool {
    let counts = s.counts[l.into(): usize];
    StackHeight(15) == if p { counts.0 } else { counts.1 }
}

// This would be unnecessary if locations depended on player
fn flip(x: Location) -> Location {
    match x {
        Board(Point(n)) => Board(Point(25 - n)),
        Home => Home,
        Bar => Bar,
    }
}
