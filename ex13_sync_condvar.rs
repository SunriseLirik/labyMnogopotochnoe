use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let state = Arc::new((Mutex::new(true), Condvar::new())); // true = очередь родителя
    let state_clone = Arc::clone(&state);

    let handle = thread::spawn(move || {
        let (lock, cvar) = &*state_clone;
        for i in 1..=10 {
            let mut parent_turn = lock.lock().unwrap();
            while *parent_turn {
                parent_turn = cvar.wait(parent_turn).unwrap();
            }
            println!("Дочерний поток: строка {}", i);
            *parent_turn = true;
            cvar.notify_one();
        }
    });

    let (lock, cvar) = &*state;
    for i in 1..=10 {
        let mut parent_turn = lock.lock().unwrap();
        while !*parent_turn {
            parent_turn = cvar.wait(parent_turn).unwrap();
        }
        println!("Родительский поток: строка {}", i);
        *parent_turn = false;
        cvar.notify_one();
    }

    handle.join().unwrap();
}
