use std::sync::Arc;
use std::thread;

// Простая реализация семафора-счётчика
struct Semaphore {
    cond: std::sync::Condvar,
    count: std::sync::Mutex<usize>,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            cond: std::sync::Condvar::new(),
            count: std::sync::Mutex::new(count),
        }
    }

    fn wait(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cond.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn post(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cond.notify_one();
    }
}

fn main() {
    let sem_parent = Arc::new(Semaphore::new(1)); // родитель начинает
    let sem_child = Arc::new(Semaphore::new(0));  // дочерний ждёт

    let sem_parent_clone = Arc::clone(&sem_parent);
    let sem_child_clone = Arc::clone(&sem_child);

    let handle = thread::spawn(move || {
        for i in 1..=10 {
            sem_child_clone.wait();
            println!("Дочерний поток: строка {}", i);
            sem_parent_clone.post();
        }
    });

    for i in 1..=10 {
        sem_parent.wait();
        println!("Родительский поток: строка {}", i);
        sem_child.post();
    }

    handle.join().unwrap();
}
