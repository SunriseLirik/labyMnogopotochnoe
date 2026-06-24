use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

const NUM_PHILOSOPHERS: usize = 5;

struct Fork {
    id: usize,
    mutex: Mutex<()>,
}

struct DiningTable {
    forks: Vec<Arc<Fork>>,
    forks_mutex: Mutex<()>,
    condvar: Condvar,
}

impl DiningTable {
    fn new() -> Self {
        let mut forks = Vec::with_capacity(NUM_PHILOSOPHERS);
        for i in 0..NUM_PHILOSOPHERS {
            forks.push(Arc::new(Fork {
                id: i,
                mutex: Mutex::new(()),
            }));
        }
        DiningTable {
            forks,
            forks_mutex: Mutex::new(()),
            condvar: Condvar::new(),
        }
    }

    fn take_forks(&self, id: usize) -> (std::sync::MutexGuard<()>, std::sync::MutexGuard<()>) {
        let left = &self.forks[id];
        let right = &self.forks[(id + 1) % NUM_PHILOSOPHERS];

        let _forks_guard = self.forks_mutex.lock().unwrap();

        loop {
            // Пробуем захватить обе вилки
            let left_lock = left.mutex.try_lock();
            let right_lock = right.mutex.try_lock();

            match (left_lock, right_lock) {
                (Ok(l), Ok(r)) => {
                    return (l, r);
                }
                (Ok(l), Err(_)) => {
                    // Освобождаем левую, ждём
                    drop(l);
                }
                (Err(_), Ok(r)) => {
                    // Освобождаем правую, ждём
                    drop(r);
                }
                (Err(_), Err(_)) => {
                    // Обе заняты, ждём
                }
            }

            // Ждём на условной переменной
            let _ = self.condvar.wait(_forks_guard).unwrap();
        }
    }

    fn put_forks(&self, _left: std::sync::MutexGuard<()>, _right: std::sync::MutexGuard<()>) {
        // Вилки освобождаются при drop
        let _forks_guard = self.forks_mutex.lock().unwrap();
        self.condvar.notify_all();
    }

    fn philosopher(&self, id: usize) {
        for _ in 0..10 {
            println!("Философ {} размышляет...", id);
            thread::sleep(Duration::from_millis(100));

            let (left, right) = self.take_forks(id);
            println!("Философ {} взял обе вилки и ест!", id);
            thread::sleep(Duration::from_millis(100));

            self.put_forks(left, right);
            println!("Философ {} положил вилки", id);
        }
    }
}

fn main() {
    let table = Arc::new(DiningTable::new());
    let mut handles = Vec::new();

    for i in 0..NUM_PHILOSOPHERS {
        let table = Arc::clone(&table);
        handles.push(thread::spawn(move || {
            table.philosopher(i);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
