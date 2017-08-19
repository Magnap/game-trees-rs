extern crate game_trees;

use game_trees::game::*;
use game_trees::mcts_hashtable;
use std::collections::HashMap;

use std::error::Error;
use std::sync::{Arc, RwLock};
use std::thread;
use std::io;

#[cfg_attr(feature = "debug", derive(Debug))]
struct Nim;

impl Game for Nim {
    type Move = u8;
    type State = (u8, bool);
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
        (1..(std::cmp::min(10, 100 - s.0) + 1)).collect()
    }

    fn points(s: &Self::State) -> Option<HashMap<Self::Player, Score>> {
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

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e)
    }
}

fn run() -> Result<(), Box<Error>> {
    let gt = Arc::new(RwLock::new(mcts_hashtable::MctsTable::<Nim>::new()));
    let s = Arc::new(RwLock::new(<Nim as Game>::new()));
    {
        let gt = gt.clone();
        let s = s.clone();
        thread::spawn(move || loop {
            let s = s.read().unwrap();
            let mut gt = gt.write().unwrap();
            for _ in 0..100 {
                gt.playout(&s)
            }
        });
    }
    println!("Let's play the 100 game. You go first:");
    let mut buf = String::new();
    loop {
        let m: <Nim as Game>::Move;
        loop {
            let s = s.read().unwrap();
            buf.clear();
            io::stdin().read_line(&mut buf)?;
            let m_opt = buf.trim().parse()?;
            if <Nim as Game>::legal_moves(&s).contains(&m_opt) {
                m = m_opt;
                break;
            } else {
                println!("That's not a legal move. Please try again:");
            }
        }
        {
            let mut s = s.write().unwrap();
            <Nim as Game>::apply(&mut s, m);
            println!("The state is now {}", s.0);
            if <Nim as Game>::finished(&*s) {
                println!("Looks like you won. Congratulations!");
                break;
            }
            let gt = gt.read().unwrap();
            let m = gt.best_choice(&s).ok_or("No moves available")?;
            println!("I'll pick {}", m);
            <Nim as Game>::apply(&mut s, *m);
            println!("The state is now {}", s.0);
            if <Nim as Game>::finished(&*s) {
                println!("Looks like I won. Too bad!");
                break;
            }
        }
    }
    Ok(())
}
