use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;

const LINES_PER_SCREEN: usize = 25;

enum NetworkMsg {
    Data(Vec<u8>),
    Done,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <url>", args[0]);
        std::process::exit(1);
    }

    let url = parse_url(&args[1]);
    let mut stream = TcpStream::connect(format!("{}:{}", url.host, url.port)).unwrap();
    
    let request = format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", url.path, url.host);
    stream.write_all(request.as_bytes()).unwrap();

    let (net_tx, net_rx): (Sender<NetworkMsg>, Receiver<NetworkMsg>) = channel();
    let (user_tx, user_rx): (Sender<()>, Receiver<()>) = channel();

    // Поток чтения из сети
    let mut stream_clone = stream.try_clone().unwrap();
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut header_skipped = false;

        loop {
            match stream_clone.read(&mut buffer) {
                Ok(0) => { let _ = net_tx.send(NetworkMsg::Done); break; }
                Ok(n) => {
                    if !header_skipped {
                        let data = String::from_utf8_lossy(&buffer[..n]);
                        if let Some(pos) = data.find("\r\n\r\n") {
                            let body = buffer[pos + 4..n].to_vec();
                            let _ = net_tx.send(NetworkMsg::Data(body));
                            header_skipped = true;
                        }
                    } else {
                        let _ = net_tx.send(NetworkMsg::Data(buffer[..n].to_vec()));
                    }
                }
                Err(_) => { let _ = net_tx.send(NetworkMsg::Done); break; }
            }
        }
    });

    // Поток взаимодействия с пользователем
    thread::spawn(move || {
        let mut input = [0u8; 1];
        loop {
            if io::stdin().read_exact(&mut input).is_ok() && input[0] == b' ' {
                let _ = user_tx.send(());
            }
        }
    });

    // Основной поток — вывод
    let mut output_buffer = Vec::new();
    let mut lines_printed = 0;
    let mut waiting_for_space = false;

    loop {
        if !waiting_for_space {
            // Проверяем данные из сети
            match net_rx.try_recv() {
                Ok(NetworkMsg::Data(data)) => {
                    output_buffer.extend_from_slice(&data);
                    print_buffered(&mut output_buffer, &mut lines_printed, &mut waiting_for_space);
                }
                Ok(NetworkMsg::Done) => break,
                Err(_) => {}
            }
        } else {
            // Ждём нажатия пробела
            match user_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(()) => {
                    waiting_for_space = false;
                    lines_printed = 0;
                    print_buffered(&mut output_buffer, &mut lines_printed, &mut waiting_for_space);
                }
                Err(_) => {
                    // Продолжаем читать сеть в буфер
                    match net_rx.try_recv() {
                        Ok(NetworkMsg::Data(data)) => output_buffer.extend_from_slice(&data),
                        Ok(NetworkMsg::Done) => break,
                        Err(_) => {}
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    if !output_buffer.is_empty() {
        io::stdout().write_all(&output_buffer).unwrap();
    }
}

struct Url {
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Url {
    let url = url.strip_prefix("http://").unwrap_or(url);
    let parts: Vec<&str> = url.splitn(2, '/').collect();
    let host_port = parts[0];
    let path = if parts.len() > 1 { format!("/{}", parts[1]) } else { "/".to_string() };

    let hp: Vec<&str> = host_port.splitn(2, ':').collect();
    let host = hp[0].to_string();
    let port = if hp.len() > 1 { hp[1].parse().unwrap_or(80) } else { 80 };

    Url { host, port, path }
}

fn print_buffered(buffer: &mut Vec<u8>, lines_printed: &mut usize, waiting: &mut bool) {
    let data = String::from_utf8_lossy(buffer);
    let lines: Vec<&str> = data.split('\n').collect();

    for (i, line) in lines.iter().enumerate() {
        if *lines_printed >= LINES_PER_SCREEN {
            *waiting = true;
            let remaining = lines[i..].join("\n");
            buffer.clear();
            buffer.extend_from_slice(remaining.as_bytes());
            println!("\n-- Press space to scroll down --");
            return;
        }
        println!("{}", line);
        *lines_printed += 1;
    }

    buffer.clear();
}
