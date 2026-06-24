use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

const MAX_QUEUE_SIZE: usize = 10;
const MAX_MSG_LEN: usize = 80;

struct MessageQueue {
    messages: Mutex<Vec<String>>,
    not_full: Condvar,
    not_empty: Condvar,
    dropped: Mutex<bool>,
}

impl MessageQueue {
    fn new() -> Self {
        MessageQueue {
            messages: Mutex::new(Vec::new()),
            not_full: Condvar::new(),
            not_empty: Condvar::new(),
            dropped: Mutex::new(false),
        }
    }

    fn mymsgput(&self, msg: &str) -> usize {
        let mut messages = self.messages.lock().unwrap();

        // Ждём, пока очередь не заполнена и не дропнута
        loop {
            let dropped = self.dropped.lock().unwrap();
            if *dropped {
                return 0;
            }
            drop(dropped);

            if messages.len() < MAX_QUEUE_SIZE {
                break;
            }
            messages = self.not_full.wait(messages).unwrap();
        }

        // Проверяем снова после пробуждения
        let dropped = self.dropped.lock().unwrap();
        if *dropped {
            return 0;
        }
        drop(dropped);

        let truncated = if msg.len() > MAX_MSG_LEN {
            &msg[..MAX_MSG_LEN]
        } else {
            msg
        };

        let len = truncated.len();
        messages.push(truncated.to_string());

        // Сигнализируем потребителям
        self.not_empty.notify_one();

        len
    }

    fn mymsgget(&self, buf: &mut [u8]) -> usize {
        let mut messages = self.messages.lock().unwrap();

        // Ждём, пока очередь не пуста и не дропнута
        loop {
            let dropped = self.dropped.lock().unwrap();
            if *dropped {
                return 0;
            }
            drop(dropped);

            if !messages.is_empty() {
                break;
            }
            messages = self.not_empty.wait(messages).unwrap();
        }

        // Проверяем снова после пробуждения
        let dropped = self.dropped.lock().unwrap();
        if *dropped {
            return 0;
        }
        drop(dropped);

        let msg = messages.remove(0);
        let copy_len = msg.len().min(buf.len());
        buf[..copy_len].copy_from_slice(&msg.as_bytes()[..copy_len]);

        // Сигнализируем производителям
        self.not_full.notify_one();

        copy_len
    }

    fn mymsgdrop(&self) {
        let mut dropped = self.dropped.lock().unwrap();
        *dropped = true;
        drop(dropped);

        // Разбудить все ожидающие
        self.not_full.notify_all();
        self.not_empty.notify_all();
    }

    fn mymsgdestroy(&self) {
        let mut messages = self.messages.lock().unwrap();
        messages.clear();
    }
}

fn main() {
    let queue = Arc::new(MessageQueue::new());

    let mut handles = vec![];

    // Два производителя
    for i in 0..2 {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            for j in 0..20 {
                let msg = format!("Producer {} message {}", i, j);
                let len = q.mymsgput(&msg);
                if len == 0 {
                    println!("Producer {}: queue dropped", i);
                    break;
                }
                println!("Producer {} sent: {} ({} bytes)", i, msg, len);
                thread::sleep(Duration::from_millis(100));
            }
        }));
    }

    // Два потребителя
    for i in 0..2 {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            let mut buf = [0u8; 100];
            loop {
                let len = q.mymsgget(&mut buf);
                if len == 0 {
                    println!("Consumer {}: queue dropped", i);
                    break;
                }
                let msg = String::from_utf8_lossy(&buf[..len]);
                println!("Consumer {} received: {}", i, msg);
                thread::sleep(Duration::from_millis(150));
            }
        }));
    }

    // Даём поработать 3 секунды, потом дропаем очередь
    thread::sleep(Duration::from_secs(3));
    println!("Dropping queue...");
    queue.mymsgdrop();

    for h in handles {
        h.join().unwrap();
    }

    queue.mymsgdestroy();
    println!("Queue destroyed");
}
