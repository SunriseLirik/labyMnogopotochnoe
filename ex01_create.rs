use std::thread;

fn main() {
    // Создание потока (атрибуты по умолчанию)
    let handle = thread::spawn(|| {
        for i in 1..=10 {
            println!("Дочерний поток: строка {}", i);
        }
    });

    for i in 1..=10 {
        println!("Родительский поток: строка {}", i);
    }

}
