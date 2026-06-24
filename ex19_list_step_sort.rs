use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Аналогично задаче 18, но с sleep(1) между перестановками
fn bubble_sort_with_delay(list: &LinkedList) {
    let _head_guard = list.head_mutex.lock().unwrap();

    loop {
        let mut swapped = false;

        let first = match &list.head {
            Some(node) => node.clone(),
            None => break,
        };

        let mut current = first;
        let mut current_guard = current.lock().unwrap();

        while let Some(ref next_arc) = current_guard.next {
            let next = next_arc.clone();
            let next_guard = next.lock().unwrap();

            if current_guard.data > next_guard.data {
                drop(current_guard);
                drop(next_guard);

                {
                    let mut c = current.lock().unwrap();
                    let mut n = next.lock().unwrap();
                    std::mem::swap(&mut c.data, &mut n.data);
                }

                swapped = true;

                // Задержка 1 секунда между перестановками
                drop(current);
                thread::sleep(Duration::from_secs(1));

                current = next;
                current_guard = current.lock().unwrap();
            } else {
                drop(current_guard);
                current = next;
                current_guard = next.lock().unwrap();
            }
        }

        if !swapped {
            break;
        }
    }
}
