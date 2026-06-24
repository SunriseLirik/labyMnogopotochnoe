use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Node {
    data: String,
    next: Option<Arc<Mutex<Node>>>,
    mutex: Mutex<()>,
}

struct LinkedList {
    head: Option<Arc<Mutex<Node>>>,
    head_mutex: Mutex<()>,
}

impl LinkedList {
    fn new() -> Self {
        LinkedList {
            head: None,
            head_mutex: Mutex::new(()),
        }
    }

    fn push_front(&mut self, data: String) {
        let new_node = Arc::new(Mutex::new(Node {
            data,
            next: self.head.take(),
            mutex: Mutex::new(()),
        }));
        self.head = Some(new_node);
    }
}

fn bubble_sort(list: &LinkedList) {
    let _head_guard = list.head_mutex.lock().unwrap();

    loop {
        let mut swapped = false;

        // Захватываем первый узел
        let first = match &list.head {
            Some(node) => node.clone(),
            None => break,
        };
        let _first_guard = first.lock().unwrap();

        let mut current = first;
        let mut current_guard = current.lock().unwrap();

        while let Some(ref next_arc) = current_guard.next {
            let next = next_arc.clone();
            let next_guard = next.lock().unwrap();

            if current_guard.data > next_guard.data {
                // Обмен данными
                // Нужно освободить оба мьютекса, обменять, потом захватить
                // Упрощённая версия: обмен через временные переменные
                drop(current_guard);
                drop(next_guard);

                let mut c = current.lock().unwrap();
                let mut n = next.lock().unwrap();
                std::mem::swap(&mut c.data, &mut n.data);
                drop(n);
                drop(c);

                swapped = true;

                current_guard = current.lock().unwrap();
            } else {
                drop(current_guard);
                current = next;
                current_guard = next_guard;
            }
        }

        if !swapped {
            break;
        }
    }
}

fn main() {
    let list = Arc::new(Mutex::new(LinkedList::new()));
    let list_clone = Arc::clone(&list);

    let handle = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(5));
            let list = list_clone.lock().unwrap();
            bubble_sort(&*list);
        }
    });

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if line.is_empty() {
            let list = list.lock().unwrap();
            println!("--- Текущее состояние списка ---");
            // print...
            println!("---------------------------------");
        } else {
            let mut list = list.lock().unwrap();
            let chunks: Vec<String> = line.as_bytes()
                .chunks(80)
                .map(|c| String::from_utf8_lossy(c).to_string())
                .collect();
            for chunk in chunks {
                list.push_front(chunk);
            }
        }
    }

    handle.join().unwrap();
}
