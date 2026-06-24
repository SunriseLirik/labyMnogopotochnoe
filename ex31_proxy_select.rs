use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, RawFd};
use libc::{select, FD_ISSET, FD_SET, FD_ZERO, fd_set};

struct CacheEntry {
    data: Vec<u8>,
    headers: Vec<u8>,
}

struct Proxy {
    cache: HashMap<String, CacheEntry>,
    clients: HashMap<RawFd, ClientState>,
}

struct ClientState {
    stream: TcpStream,
    buffer: Vec<u8>,
    state: ClientPhase,
    server_fd: Option<RawFd>,
    request_url: String,
}

enum ClientPhase {
    ReadingRequest,
    ConnectingToServer,
    Forwarding,
    ServingFromCache,
    Done,
}

impl Proxy {
    fn new() -> Self {
        Proxy {
            cache: HashMap::new(),
            clients: HashMap::new(),
        }
    }

    fn handle_request(&mut self, fd: RawFd, read_fds: &mut fd_set) {
        let state = self.clients.get_mut(&fd).unwrap();

        match state.state {
            ClientPhase::ReadingRequest => {
                let mut buf = [0u8; 4096];
                match state.stream.read(&mut buf) {
                    Ok(0) => { state.state = ClientPhase::Done; return; }
                    Ok(n) => {
                        state.buffer.extend_from_slice(&buf[..n]);
                        
                        // Проверяем полный HTTP-запрос
                        if let Some(pos) = find_double_crlf(&state.buffer) {
                            let request = String::from_utf8_lossy(&state.buffer[..pos]);
                            let url = extract_url(&request);
                            state.request_url = url.clone();

                            // Проверяем кэш
                            if let Some(entry) = self.cache.get(&url) {
                                // Отдаём из кэша
                                state.stream.write_all(&entry.headers).unwrap();
                                state.stream.write_all(&entry.data).unwrap();
                                state.state = ClientPhase::ServingFromCache;
                            } else {
                                // Подключаемся к серверу
                                match TcpStream::connect(format!("{}:80", extract_host(&url))) {
                                    Ok(server) => {
                                        let server_fd = server.as_raw_fd();
                                        server.set_nonblocking(true).unwrap();
                                        server.write_all(&state.buffer).unwrap();
                                        state.server_fd = Some(server_fd);
                                        state.state = ClientPhase::Forwarding;
                                        
                                        // Сохраняем серверное соединение
                                        // (упрощённо — в реальности нужен отдельный map)
                                    }
                                    Err(_) => {
                                        state.state = ClientPhase::Done;
                                    }
                                }
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => { state.state = ClientPhase::Done; }
                }
            }
            _ => {}
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
    listener.set_nonblocking(true).unwrap();

    let mut proxy = Proxy::new();
    let listen_fd = listener.as_raw_fd();

    loop {
        let mut read_fds: fd_set = unsafe { std::mem::zeroed() };
        let mut max_fd = listen_fd;

        unsafe { FD_ZERO(&mut read_fds); FD_SET(listen_fd, &mut read_fds); }

        for (fd, state) in &proxy.clients {
            let fd = *fd;
            max_fd = max_fd.max(fd);
            unsafe { FD_SET(fd, &mut read_fds); }
            
            if let Some(srv) = state.server_fd {
                max_fd = max_fd.max(srv);
                unsafe { FD_SET(srv, &mut read_fds); }
            }
        }

        let timeout = libc::timeval { tv_sec: 1, tv_usec: 0 };
        let result = unsafe {
            select(max_fd + 1, &mut read_fds, std::ptr::null_mut(), std::ptr::null_mut(), &timeout)
        };

        if result < 0 { break; }

        // Новые соединения
        if unsafe { FD_ISSET(listen_fd, &read_fds) } {
            if let Ok((stream, _)) = listener.accept() {
                stream.set_nonblocking(true).unwrap();
                let fd = stream.as_raw_fd();
                proxy.clients.insert(fd, ClientState {
                    stream,
                    buffer: Vec::new(),
                    state: ClientPhase::ReadingRequest,
                    server_fd: None,
                    request_url: String::new(),
                });
            }
        }

        // Обработка клиентов
        let mut to_remove = Vec::new();
        for fd in proxy.clients.keys().copied().collect::<Vec<_>>() {
            if unsafe { FD_ISSET(fd, &mut read_fds) } {
                proxy.handle_request(fd, &mut read_fds);
            }
            
            if let Some(state) = proxy.clients.get(&fd) {
                if matches!(state.state, ClientPhase::Done) {
                    to_remove.push(fd);
                }
            }
        }

        for fd in to_remove {
            proxy.clients.remove(&fd);
        }
    }
}

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
