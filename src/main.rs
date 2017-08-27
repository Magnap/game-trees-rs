#![feature(type_ascription)]

extern crate game_trees;
extern crate itertools;

use game_trees::game::GameState;
use game_trees::game::backgammon;
use backgammon::Backgammon;
use backgammon::{Point, board};
use backgammon::Location::*;
use game_trees::mcts_hashtable::MctsTable;

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::io;
use std::io::Write;
use std::time::Duration;
use itertools::Itertools;

type BoxResult<T> = Result<T, Box<Error>>;

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e)
    }
}

fn run() -> BoxResult<()> {
    let mut s = Backgammon::new();
    let gt = Arc::new(Mutex::new(MctsTable::<Backgammon>::with_state(s.clone())));
    println!("Let's play Backgammon. Do you want to go first? If so write \"yes\"");
    let mut buf = String::new();
    buf.clear();
    io::stdin().read_line(&mut buf)?;
    let mut human_turn = buf.trim() == "yes";
    let s_ref = Arc::new(Mutex::new(s.clone()));
    // TODO change mcts_hashtable to use a concurrent hashtable,
    // allowing for more workers
    for _ in 0..1 {
        let gt = gt.clone();
        let s_ref = s_ref.clone();
        let mut s = s.clone();
        let mut wait = false;
        thread::spawn(move || loop {
            {
                let s_locked = s_ref.lock().unwrap();
                if *s_locked != s {
                    s = s_locked.clone();
                    wait = false;
                }
            }
            if !wait {
                let mut gt = gt.lock().unwrap();
                for _ in 0..32 {
                    gt.playout(&s, 400);
                }
            } else {
                thread::sleep(Duration::from_millis(10));
            }
            // This constant depends on game state size
            // and was chosen to remain at a reasonable level of memory use
            wait = gt.lock().unwrap().0.len() > 2usize.pow(20);
            // TODO find a better way to ensures main thread can get the lock
            // within a reasonable timeframe
            thread::sleep(Duration::from_millis(1));
        });
    }
    loop {
        let old_state = s.clone();
        let d = dice_turn(&mut buf);
        apply(d, &mut s, &s_ref);
        gc(&gt, &s, old_state);
        let old_state = s.clone();
        if human_turn {
            let m = move_turn(&mut buf, &s);
            apply(m, &mut s, &s_ref);
            if s.finished() {
                println!("Looks like you won. Congratulations!");
                break;
            }
        } else {
            let m = computer_turn(&*gt, &mut s)?;
            print!("My move is: ");
            for (l, n) in m.clone().unwrap() {
                print!("{},{} ", l, n);
            }
            println!();
            apply(m, &mut s, &s_ref);
            if s.finished() {
                println!("Looks like I won. Too bad!");
                break;
            }
        }
        human_turn = !human_turn;
        print_expectations(&s, &*gt);
        print_state(&s);
        gc(&gt, &s, old_state);
    }
    Ok(())
}

// TODO merge this into mcts_hashtable
fn gc(gt: &Arc<Mutex<MctsTable<Backgammon>>>, new: &Backgammon, old: Backgammon) {
    {
        let gt = gt.clone();
        let new = new.clone();
        thread::spawn(move || {
            let mut gt = gt.lock().unwrap();
            let old_meta = gt.0[&old].clone();
            for (_, (s, _)) in old_meta.moves {
                if s != new {
                    gt.garbage_collect(&s);
                }
            }
        });
    }
}

fn apply(m: <Backgammon as GameState>::Move, s: &mut Backgammon, s_ref: &Arc<Mutex<Backgammon>>) {
    s.apply(m);
    {
        *s_ref.lock().unwrap() = s.clone();
    }
}

fn dice_turn(buf: &mut String) -> <Backgammon as GameState>::Move {
    println!("What do the dice show?");
    println!("Write the lowest number, then a space, then the highest.");
    loop {
        match dice_turn_(buf) {
            Ok(pair) => return Err(pair),
            Err(e) => println!("{}", e),
        }
    }
}

// Ugly parsing
fn dice_turn_(mut buf: &mut String) -> BoxResult<backgammon::Roll> {
    buf.clear();
    io::stdin().read_line(&mut buf)?;
    buf.pop().ok_or("Write something!")?;
    let mut singleton: String = "".into();
    singleton.push(buf.pop().ok_or("C'mon, write at least one digit!")?);
    let x = singleton.parse()?;
    buf.pop().ok_or("You need to write both numbers")?;
    let mut singleton: String = "".into();
    singleton.push(buf.pop().ok_or("Might as well write the other digit")?);
    let y = singleton.parse()?;
    if !(x >= y && y > 0 && x > 0 && x < 7) {
        Err("Not a valid dice roll")?;
    }
    Ok((x, y))
}

fn move_turn(buf: &mut String, s: &Backgammon) -> <Backgammon as GameState>::Move {
    println!("What's your move?");
    println!("Legal moves should be");
    for m in s.legal_moves() {
        println!("{}", format_sequence(&m.unwrap()))
    }
    println!("The format is (location,places moved )+");
    loop {
        match move_turn_(buf, s) {
            Ok(Ok(m)) => return Ok(m),
            Err(e) => println!("{}", e),
            _ => panic!("move_turn_ returned illegal move"),
        }
    }
}

// TODO don't require user to canonicalize
fn move_turn_(mut buf: &mut String, s: &Backgammon) -> BoxResult<<Backgammon as GameState>::Move> {
    buf.clear();
    io::stdin().read_line(&mut buf)?;
    let m = Ok(parse_moves(buf)?);
    if !s.legal_moves().contains(&m) {
        Err("That's not a legal move. Please try again.")?;
    }
    Ok(m)
}

fn parse_moves(buf: &str) -> BoxResult<Vec<backgammon::SingleMove>> {
    let res: BoxResult<Vec<_>> = buf.split_whitespace()
        .map(|s| {
            let s = s.replace(",", " ");
            let ss: Vec<_> = s.split_whitespace().collect();
            let (l_s, n_s) = (
                ss.get(0).ok_or("Couldn't get location")?,
                ss.get(1).ok_or("Couldn't get amount")?,
            );
            let l = parse_location(l_s)?;
            let n = n_s.parse()?;
            Ok((l, n))
        })
        .collect();
    Ok(res?)
}

// Should be the inverse of the Display impl
fn parse_location(s: &str) -> BoxResult<backgammon::Location> {
    Ok(match s {
        "home" => Home,
        "bar" => Bar,
        s => Board(Point(s.parse()?)),
    })
}

fn computer_turn(
    gt: &Mutex<MctsTable<Backgammon>>,
    s: &mut Backgammon,
) -> BoxResult<<Backgammon as GameState>::Move> {
    let mut countdown = 5;
    let moves = s.legal_moves().len();
    print!("Considering my next move..");
    io::stdout().flush()?;
    thread::sleep(Duration::from_millis(2000));
    while !gt.lock()
        .unwrap()
        .0
        .get(s)
        .map(|x| (*x).playouts)
        // constant chosen as a balance between waiting time and strength of play
        .unwrap_or(0) >= (32 * moves as u32)
    {
        if countdown == 0 {
            break;
        } else {
            print!(".");
            io::stdout().flush()?;
            countdown -= 1;
            thread::sleep(Duration::from_millis(1000));
        }
    }
    println!();
    Ok(gt.lock()
        .unwrap()
        .best_choice(s)
        .ok_or("No moves available")?)
}

fn format_sequence(ms: &[backgammon::SingleMove]) -> String {
    ms.iter()
        .map(|m| format_move(m))
        .intersperse(" ".to_string())
        .collect()
}

fn format_move(m: &backgammon::SingleMove) -> String {
    let &(l, n) = m;
    format!("{},{}", l, n)
}

// TODO find a better way to format this
fn print_state(s: &Backgammon) {
    let mut b = board();
    b.push(Bar);
    b.push(Home);
    println!("The current state should be:");
    for &l in &b {
        let count = s.counts[l.into(): usize];
        println!("{}: ({}, {})", &l, count.0, count.1)
    }
}

fn print_expectations(s: &Backgammon, gt: &Mutex<MctsTable<Backgammon>>) {
    let meta = &gt.lock().unwrap().0[s];
    println!(
        "Expected score {} over {} playouts",
        meta.scoreboard[&Some(!s.player)] / meta.playouts as f64,
        meta.playouts
    );
}
