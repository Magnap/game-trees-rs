extern crate game_trees;
extern crate chashmap;

use game_trees::game::Game;
use game_trees::game::nim::Nim;
use game_trees::mcts_hashtable;

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::io;
use std::io::Write;
use std::time::Duration;

#[cfg_attr(feature = "debug", derive(Debug))]

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e)
    }
}

fn run() -> Result<(), Box<Error>> {
    let gt = Arc::new(mcts_hashtable::MctsTable::<Nim>::new());
    let mut s = <Nim as Game>::new();
    let s_ref = Arc::new(Mutex::new(s));
    for _ in 0..2 {
        let gt = gt.clone();
        let s_ref = s_ref.clone();
        let mut s = s;
        thread::spawn(move || loop {
            {
                let s_locked = s_ref.lock().unwrap();
                if *s_locked != s {
                    s = *s_locked;
                }
            }
            for _ in 0..100 {
                gt.playout(&s);
            }
        });
    }
    println!("Let's play the 100 game. You go first.");
    let mut buf = String::new();
    loop {
        let m: <Nim as Game>::Move;
        loop {
            println!("What's your move?");
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
        <Nim as Game>::apply(&mut s, m);
        println!("The state is now {}", s.0);
        if <Nim as Game>::finished(&s) {
            println!("Looks like you won. Congratulations!");
            break;
        }
        {
            *s_ref.lock().unwrap() = s;
        }
        let mut needed_time = false;
        while gt.0.get(&s).map(|x| x.playouts).unwrap_or(0) <
            (10 * <Nim as Game>::legal_moves(&s).len() as u32)
        {
            if needed_time {
                print!(".");
                io::stdout().flush()?;
            } else {
                print!("I need some time to consider my next move.");
                io::stdout().flush()?;
                needed_time = true;
            }
            thread::sleep(Duration::from_millis(1000));
        }
        if needed_time {
            println!();
        };
        let m = gt.best_choice(&s).ok_or("No moves available")?;
        println!("I'll pick {}", m);
        <Nim as Game>::apply(&mut s, m);
        println!("The state is now {}", s.0);
        if <Nim as Game>::finished(&s) {
            println!("Looks like I won. Too bad!");
            break;
        }
        {
            *s_ref.lock().unwrap() = s;
        }
    }
    Ok(())
}
