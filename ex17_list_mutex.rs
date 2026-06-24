use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Node {
    data: String,
    next: Option<Box<Node>>,
}

struct LinkedList {
    head: Option<Box<Node>>,
}

impl LinkedList {
    fn new() -> Self {
        LinkedList { head: None }
    }

    fn push_front(&mut self, data: String) {
        let new_node = Box::new(Node {
            data,
            next: self.head.take(),
        });
        self.head = Some(new_node);
    }

    fn bubble_sort(&mut self) {
        if self.head.is_none() {
            return;
        }

        let mut swapped = true;
        while swapped {
            swapped = false;
            let mut current = &mut self.head;
            while let Some(ref mut node) = *current {
                if let Some(ref mut next) = node.next {
                    if node.data > next.data {
                        // Обмен данными (не указателями — проще с мьютексом)
                        std::mem::swap(&mut node.data, &mut next.data);
                        swapped = true;
                    }
                }
                current = &mut current.as_mut().unwrap().next;
            }
        }
    }

    fn print(&self) {
        let mut current = &self.head;
        while let Some(ref node) = *current {
            println!("{}", node.data);
            current = &node.next;
        }
    }
}

fn main() {
    let list = Arc::new(Mutex::new(LinkedList::new()));
    let list_clone = Arc::clone(&list);

    // Дочерний поток — сортировка каждые 5 секунд
    let handle = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(5));
            let mut list = list_clone.lock().unwrap();
            list.bubble_sort();
        }
    });

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if line.is_empty() {
            let list = list.lock().unwrap();
            println!("--- Текущее состояние списка ---");
            list.print();
            println!("---------------------------------");
        } else {
            let mut list = list.lock().unwrap();
            // Разрезаем строки длиннее 80 символов
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
