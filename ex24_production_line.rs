use std::sync::Arc;
use std::thread;
use std::time::Duration;

// Семафор-счётчик (упрощённая реализация)
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
    // Семафоры для деталей
    let sem_a = Arc::new(Semaphore::new(0));
    let sem_b = Arc::new(Semaphore::new(0));
    let sem_c = Arc::new(Semaphore::new(0));
    let sem_module = Arc::new(Semaphore::new(0));

    let sem_a_clone = Arc::clone(&sem_a);
    let sem_b_clone = Arc::clone(&sem_b);
    let sem_c_clone = Arc::clone(&sem_c);
    let sem_module_clone = Arc::clone(&sem_module);

    // Поток для детали A (1 секунда)
    let handle_a = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            println!("Деталь A изготовлена");
            sem_a_clone.post();
        }
    });

    // Поток для детали B (2 секунды)
    let handle_b = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(2));
            println!("Деталь B изготовлена");
            sem_b_clone.post();
        }
    });

    // Поток для детали C (3 секунды)
    let handle_c = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(3));
            println!("Деталь C изготовлена");
            sem_c_clone.post();
        }
    });

    // Поток для модуля (A + B)
    let handle_module = thread::spawn(move || {
        loop {
            sem_a.wait();
            sem_b.wait();
            println!("Модуль (A + B) собран");
            sem_module.post();
        }
    });

    // Главный поток — сборка винтика (модуль + C)
    loop {
        sem_module.wait();
        sem_c.wait();
        println!("=== ВИНТИК СОБРАН ===\n");
    }
}
