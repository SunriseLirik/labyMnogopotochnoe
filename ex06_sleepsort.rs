use std::thread;
use std::time::Duration;

fn main() {
    let mut input = String::new();
    let mut lines = Vec::new();

    // Читаем stdin
    while std::io::stdin().read_line(&mut input).unwrap() > 0 {
        let line = input.trim_end().to_string();
        if !line.is_empty() {
            lines.push(line);
        }
        input.clear();
    }

    let mut handles = Vec::new();

    for line in lines {
        let len = line.len();
        let handle = thread::spawn(move || {
            // sleep пропорционально длине строки (в микросекундах)
            // 100000 мкс = 0.1 сек на символ
            let sleep_us = (len as u64) * 100000;
            thread::sleep(Duration::from_micros(sleep_us));
            println!("{}", line);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
