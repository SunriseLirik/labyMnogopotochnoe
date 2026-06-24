use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

// Структура, выполняющая cleanup при выходе из области видимости
struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        println!("Дочерний поток: я завершаюсь! (cleanup handler)");
    }
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    let handle = thread::spawn(move || {
        // Cleanup выполнится при любом выходе из функции
        let _guard = CleanupGuard;

        while running_clone.load(Ordering::Relaxed) {
            println!("Дочерний поток: работаю...");
            thread::sleep(Duration::from_millis(500));
        }
    });

    // Ждём 2 секунды
    thread::sleep(Duration::from_secs(2));

    println!("Родитель: прерываю дочерний поток!");
    running.store(false, Ordering::Relaxed);

    handle.join().unwrap();
    println!("Родитель: дочерний поток завершён");
}
