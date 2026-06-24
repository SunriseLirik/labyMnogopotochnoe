use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Node {
    data: String,
    next: Option<Box<Node>>,
}

struct SortedList {
    head: Option<Box<Node>>,
}

impl SortedList {
    fn new() -> Self {
        SortedList { head: None }
    }

    fn insert(&mut self, data: String) {
        // Вставка в конец (порядок определяется sleepsort)
        let new_node = Box::new(Node { data, next: None });

        if self.head.is_none() {
            self.head = Some(new_node);
            return;
        }

        let mut current = &mut self.head;
        while let Some(ref mut node) = *current {
            if node.next.is_none() {
                node.next = Some(new_node);
                return;
            }
            current = &mut current.as_mut().unwrap().next;
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
    let mut input = String::new();
    let mut lines = Vec::new();

    while std::io::stdin().read_line(&mut input).unwrap() > 0 {
        let line = input.trim_end().to_string();
        if !line.is_empty() {
            lines.push(line);
        }
        input.clear();
    }

    let sorted_list = Arc::new(Mutex::new(SortedList::new()));
    let mut handles = Vec::new();

    for line in lines {
        let len = line.len();
        let list_clone = Arc::clone(&sorted_list);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_micros((len as u64) * 100000));
            
            let mut list = list_clone.lock().unwrap();
            list.insert(line);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let list = sorted_list.lock().unwrap();
    println!("--- Отсортированный список ---");
    list.print();
}
