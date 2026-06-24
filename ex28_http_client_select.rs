use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::unix::io::AsRawFd;
use libc::{select, FD_ISSET, FD_SET, FD_ZERO, fd_set};

const LINES_PER_SCREEN: usize = 25;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <url>", args[0]);
        std::process::exit(1);
    }

    let url = &args[1];
    // Упрощённый парсинг URL: http://host:port/path
    let url = url.strip_prefix("http://").unwrap_or(url);
    let parts: Vec<&str> = url.splitn(2, '/').collect();
    let host_port = parts[0];
    let path = if parts.len() > 1 { format!("/{}", parts[1]) } else { "/".to_string() };

    let host_port_parts: Vec<&str> = host_port.splitn(2, ':').collect();
    let host = host_port_parts[0];
    let port: u16 = if host_port_parts.len() > 1 {
        host_port_parts[1].parse().unwrap_or(80)
    } else {
        80
    };

    let mut stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
    let request = format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", path, host);
    stream.write_all(request.as_bytes()).unwrap();

    let stream_fd = stream.as_raw_fd();
    let stdin_fd = io::stdin().as_raw_fd();

    let mut buffer = [0u8; 4096];
    let mut output_buffer = Vec::new();
    let mut lines_printed = 0;
    let mut waiting_for_space = false;
    let mut header_skipped = false;

    loop {
        let mut read_fds: fd_set = unsafe { std::mem::zeroed() };
        unsafe { FD_ZERO(&mut read_fds); }
        unsafe { FD_SET(stream_fd, &mut read_fds); }
        if waiting_for_space {
            unsafe { FD_SET(stdin_fd, &mut read_fds); }
        }

        let max_fd = stream_fd.max(stdin_fd);
        let timeout = libc::timeval { tv_sec: 0, tv_usec: 100000 };

        let result = unsafe {
            select(max_fd + 1, &mut read_fds, std::ptr::null_mut(), std::ptr::null_mut(), &timeout)
        };

        if result < 0 {
            break;
        }

        // Данные из сети
        if unsafe { FD_ISSET(stream_fd, &read_fds) } {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if !header_skipped {
                        let data = String::from_utf8_lossy(&buffer[..n]);
                        if let Some(pos) = data.find("\r\n\r\n") {
                            output_buffer.extend_from_slice(&buffer[pos + 4..n]);
                            header_skipped = true;
                        }
                    } else {
                        output_buffer.extend_from_slice(&buffer[..n]);
                    }

                    if !waiting_for_space {
                        print_buffered(&mut output_buffer, &mut lines_printed, &mut waiting_for_space);
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => break,
            }
        }

        // Ввод пользователя
        if waiting_for_space && unsafe { FD_ISSET(stdin_fd, &mut read_fds) } {
            let mut input = [0u8; 1];
            if io::stdin().read_exact(&mut input).is_ok() {
                if input[0] == b' ' {
                    waiting_for_space = false;
                    lines_printed = 0;
                    print_buffered(&mut output_buffer, &mut lines_printed, &mut waiting_for_space);
                }
            }
        }
    }

    // Вывод оставшихся данных
    if !output_buffer.is_empty() {
        io::stdout().write_all(&output_buffer).unwrap();
    }
}

fn print_buffered(buffer: &mut Vec<u8>, lines_printed: &mut usize, waiting: &mut bool) {
    let data = String::from_utf8_lossy(buffer);
    let lines: Vec<&str> = data.split('\n').collect();

    for line in &lines {
        if *lines_printed >= LINES_PER_SCREEN {
            *waiting = true;
            // Сохраняем невыведенные данные
            let remaining = lines[*lines_printed..].join("\n");
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
