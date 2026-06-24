use std::thread;

fn main() {
    let handle = thread::spawn(|| {
        for i in 1..=10 {
            println!("Дочерний поток: строка {}", i);
        }
    });

    // Ждём завершения дочернего потока
    handle.join().unwrap();

    // Вывод родительского потока после завершения дочернего
    for i in 1..=10 {
        println!("Родительский поток: строка {}", i);
    }
}
