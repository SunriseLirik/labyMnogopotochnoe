use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

const MAX_QUEUE_SIZE: usize = 10;
const MAX_MSG_LEN: usize = 80;

struct MessageQueue {
    messages: Mutex<Vec<String>>,
    // Семафор: сколько сообщений доступно для чтения
    items: Semaphore,
    // Семафор: сколько мест свободно для записи
    spaces: Semaphore,
    // Мьютекс для защиты самой очереди
    mutex: Mutex<()>,
    dropped: Mutex<bool>,
}

struct Semaphore {
    cond: Condvar,
    count: Mutex<usize>,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            cond: Condvar::new(),
            count: Mutex::new(count),
        }
    }

    fn wait(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cond.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn post(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cond.notify_one();
    }
}

impl MessageQueue {
    fn new() -> Self {
        MessageQueue {
            messages: Mutex::new(Vec::new()),
            items: Semaphore::new(0),
            spaces: Semaphore::new(MAX_QUEUE_SIZE),
            mutex: Mutex::new(()),
            dropped: Mutex::new(false),
        }
    }

    fn mymsgput(&self, msg: &str) -> usize {
        let dropped = self.dropped.lock().unwrap();
        if *dropped {
            return 0;
        }
        drop(dropped);

        // Ждём свободного места
        self.spaces.wait();

        let dropped = self.dropped.lock().unwrap();
        if *dropped {
            self.spaces.post(); // возвращаем семафор
            return 0;
        }
        drop(dropped);

        let _guard = self.mutex.lock().unwrap();

        let truncated = if msg.len() > MAX_MSG_LEN {
            &msg[..MAX_MSG_LEN]
        } else {
            msg
        };

        let len = truncated.len();
        self.messages.lock().unwrap().push(truncated.to_string());

        self.items.post();

        len
    }

    fn mymsgget(&self, buf: &mut [u8]) -> usize {
        let dropped = self.dropped.lock().unwrap();
        if *dropped {
            return 0;
        }
        drop(dropped);

        // Ждём сообщения
        self.items.wait();

        let dropped = self.dropped.lock().unwrap();
        if *dropped {
            self.items.post(); // возвращаем семафор
            return 0;
        }
        drop(dropped);

        let _guard = self.mutex.lock().unwrap();

        let msg = self.messages.lock().unwrap().remove(0);
        let copy_len = msg.len().min(buf.len());

        buf[..copy_len].copy_from_slice(&msg.as_bytes()[..copy_len]);

        self.spaces.post();

        copy_len
    }

    fn mymsgdrop(&self) {
        let mut dropped = self.dropped.lock().unwrap();
        *dropped = true;
        drop(dropped);

        // Разблокируем все ожидающие
        for _ in 0..MAX_QUEUE_SIZE * 2 {
            self.items.post();
            self.spaces.post();
        }
    }

    fn mymsgdestroy(&self) {
        // Очистка ресурсов
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
