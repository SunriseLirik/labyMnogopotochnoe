use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Мьютекс для родителя (начинает захваченным)
    let parent_mutex = Arc::new(Mutex::new(()));
    let child_mutex = Arc::new(Mutex::new(()));

    // Захватываем родительский мьютекс до создания потока
    let parent_guard = parent_mutex.lock().unwrap();

    let parent_mutex_clone = Arc::clone(&parent_mutex);
    let child_mutex_clone = Arc::clone(&child_mutex);

    let handle = thread::spawn(move || {
        for i in 1..=10 {
            // Дочерний ждёт своей очереди
            let _guard = child_mutex_clone.lock().unwrap();
            println!("Дочерний поток: строка {}", i);
            // Сигнализируем родителю
            drop(parent_mutex_clone.lock().unwrap());
        }
    });

    // Даём дочернему потоку время на старт
    thread::yield_now();

    for i in 1..=10 {
        println!("Родительский поток: строка {}", i);
        // Освобождаем родительский мьютекс, позволяя дочернему работать
        drop(parent_guard);
        // Ждём, пока дочерний освободит child_mutex
        let _guard = child_mutex.lock().unwrap();
    }

    handle.join().unwrap();
}
