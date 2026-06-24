use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const NUM_PHILOSOPHERS: usize = 5;

struct Fork {
    id: usize,
    mutex: Mutex<()>,
}

struct DiningTable {
    forks: Vec<Arc<Fork>>,
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
        DiningTable { forks }
    }

    fn philosopher(&self, id: usize) {
        let left_fork = &self.forks[id];
        let right_fork = &self.forks[(id + 1) % NUM_PHILOSOPHERS];

        // Решение deadlock: последний философ берёт вилки в обратном порядке
        let (first, second) = if id == NUM_PHILOSOPHERS - 1 {
            (right_fork, left_fork)
        } else {
            (left_fork, right_fork)
        };

        for _ in 0..10 {
            // Размышление
            println!("Философ {} размышляет...", id);
            thread::sleep(Duration::from_millis(100));

            // Берём первую вилку
            let _first = first.mutex.lock().unwrap();
            println!("Философ {} взял вилку {}", id, first.id);

            // Берём вторую вилку
            let _second = second.mutex.lock().unwrap();
            println!("Философ {} взял вилку {}", id, second.id);

            // Едим
            println!("Философ {} ест!", id);
            thread::sleep(Duration::from_millis(100));

            // Вилки освобождаются автоматически при выходе из scope (Drop)
            println!("Философ {} положил вилки", id);
        }
    }
}

fn main() {
    let table = Arc::new(DiningTable::new());
    let mut handles = Vec::new();

    for i in 0..NUM_PHILOSOPHERS {
        let table = Arc::clone(&table);
        let handle = thread::spawn(move || {
            table.philosopher(i);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Все философы наелись!");
}
