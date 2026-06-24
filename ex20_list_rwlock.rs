use std::sync::{Arc, RwLock};

struct Node {
    data: String,
    next: Option<Arc<RwLock<Node>>>,
}

struct LinkedList {
    head: Option<Arc<RwLock<Node>>>,
}

impl LinkedList {
    fn push_front(&mut self, data: String) {
        let new_node = Arc::new(RwLock::new(Node {
            data,
            next: self.head.take(),
        }));
        self.head = Some(new_node);
    }

    fn bubble_sort(&mut self) {
        // Сортировка требует записи — захватываем write lock
        loop {
            let mut swapped = false;
            // ... реализация с RwLock вместо Mutex
            if !swapped { break; }
        }
    }

    fn print(&self) {
        // Чтение — read lock
        let mut current = &self.head;
        while let Some(ref node) = *current {
            let guard = node.read().unwrap();
            println!("{}", guard.data);
            current = &guard.next;
        }
    }
}
