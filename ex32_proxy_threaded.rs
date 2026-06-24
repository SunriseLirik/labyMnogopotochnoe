use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

struct CacheEntry {
    data: Vec<u8>,
}

struct Proxy {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl Proxy {
    fn new() -> Self {
        Proxy {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn handle_client(&self, mut client: TcpStream) {
        let mut request = Vec::new();
        let mut buf = [0u8; 4096];

        // Читаем HTTP-запрос
        loop {
            match client.read(&mut buf) {
                Ok(0) => return,
                Ok(n) => {
                    request.extend_from_slice(&buf[..n]);
                    if find_double_crlf(&request).is_some() {
                        break;
                    }
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

                // Сохраняем в кэш
                let mut cache = self.cache.lock().unwrap();
                cache.insert(url, CacheEntry { data: response });
            }
            Err(_) => {
                let error = "HTTP/1.0 502 Bad Gateway\r\n\r\n";
                client.write_all(error.as_bytes()).unwrap();
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <port>", args[0]);
        std::process::exit(1);
    }

    let port: u16 = args[1].parse().unwrap();
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    let proxy = Arc::new(Proxy::new());

    for stream in listener.incoming() {
        match stream {
            Ok(client) => {
                let proxy = Arc::clone(&proxy);
                thread::spawn(move || {
                    proxy.handle_client(client);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}

// Вспомогательные функции (те же, что в задаче 31)
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
