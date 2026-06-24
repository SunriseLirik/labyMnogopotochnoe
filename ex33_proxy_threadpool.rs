use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct CacheEntry {
    data: Vec<u8>,
}

struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: std::sync::mpsc::Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = std::sync::mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for _ in 0..size {
            let receiver = Arc::clone(&receiver);
            workers.push(thread::spawn(move || {
                loop {
                    let job = receiver.lock().unwrap().recv();
                    match job {
                        Ok(job) => job(),
                        Err(_) => break,
                    }
                }
            }));
        }

        ThreadPool { workers, sender }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender.send(Box::new(f)).unwrap();
    }
}

struct Proxy {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    pool: ThreadPool,
    pending_connections: Arc<Mutex<VecDeque<TcpStream>>>,
    condvar: Arc<Condvar>,
}

impl Proxy {
    fn new(pool_size: usize) -> Self {
        let proxy = Proxy {
            cache: Arc::new(Mutex::new(HashMap::new())),
            pool: ThreadPool::new(pool_size),
            pending_connections: Arc::new(Mutex::new(VecDeque::new())),
            condvar: Arc::new(Condvar::new()),
        };

        // Диспетчер: распределяет соединения по рабочим потокам
        let pending = Arc::clone(&proxy.pending_connections);
        let condvar = Arc::clone(&proxy.condvar);
        let cache = Arc::clone(&proxy.cache);

        thread::spawn(move || {
            loop {
                let client = {
                    let mut queue = pending.lock().unwrap();
                    while queue.is_empty() {
                        queue = condvar.wait(queue).unwrap();
                    }
                    queue.pop_front().unwrap()
                };

                let cache = Arc::clone(&cache);
                // Передаём работу в пул — но здесь упрощённо
                // В реальности нужен channel в пул
            }
        });

        proxy
    }

    fn handle_client(&self, mut client: TcpStream) {
        let mut request = Vec::new();
        let mut buf = [0u8; 4096];

        loop {
            match client.read(&mut buf) {
                Ok(0) => return,
                Ok(n) => {
                    request.extend_from_slice(&buf[..n]);
                    if find_double_crlf(&request).is_some() { break; }
                }
                Err(_) => return,
            }
        }

        let request_str = String::from_utf8_lossy(&request);
        let url = extract_url(&request_str);
        let host = extract_host(&url);

        // Проверяем кэш
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(&url) {
                client.write_all(&entry.data).unwrap();
                return;
            }
        }

        // Запрос к серверу
        match TcpStream::connect(format!("{}:80", host)) {
            Ok(mut server) => {
                server.write_all(&request).unwrap();

                let mut response = Vec::new();
                loop {
                    match server.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            response.extend_from_slice(&buf[..n]);
                            client.write_all(&buf[..n]).unwrap();
                        }
                        Err(_) => break,
                    }
                }

                let mut cache = self.cache.lock().unwrap();
                cache.insert(url, CacheEntry { data: response });
            }
            Err(_) => {
                client.write_all(b"HTTP/1.0 502 Bad Gateway\r\n\r\n").unwrap();
            }
        }
    }

    fn accept_connections(&self, listener: TcpListener) {
        for stream in listener.incoming() {
            if let Ok(client) = stream {
                let mut queue = self.pending_connections.lock().unwrap();
                queue.push_back(client);
                self.condvar.notify_one();
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <port> <pool_size>", args[0]);
        std::process::exit(1);
    }

    let port: u16 = args[1].parse().unwrap();
    let pool_size: usize = args[2].parse().unwrap();

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    let proxy = Arc::new(Proxy::new(pool_size));

    // Главный поток — accept + select/poll для распределения
    let proxy_clone = Arc::clone(&proxy);
    thread::spawn(move || {
        proxy_clone.accept_connections(listener);
    });

    // Основной поток с select для управления соединениями
    // (упрощённая версия — в реальности нужен полный цикл select)
    loop {
        thread::park();
    }
}

// Вспомогательные функции
fn find_double_crlf(data: &[u8]) -> Option<usize> {
    let pattern = b"\r\n\r\n";
    data.windows(4).position(|w| w == pattern).map(|p| p + 4)
}

fn extract_url(request: &str) -> String {
    let lines: Vec<&str> = request.lines().collect();
    if let Some(first) = lines.first() {
        let parts: Vec<&str> = first.split_whitespace().collect();
        if parts.len() >= 2 {
            return parts[1].to_string();
        }
    }
    String::new()
}

fn extract_host(url: &str) -> String {
    url.strip_prefix("http://")
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_string()
}
