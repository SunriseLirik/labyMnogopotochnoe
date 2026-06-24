use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, RawFd};
use libc::{select, FD_ISSET, FD_SET, FD_ZERO, fd_set};

fn set_nonblocking(fd: RawFd) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <listen_port> <target_host> <target_port>", args[0]);
        std::process::exit(1);
    }

    let listen_port: u16 = args[1].parse().unwrap();
    let target_host = &args[2];
    let target_port: u16 = args[3].parse().unwrap();

    let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listen_fd = listener.as_raw_fd();

    let mut clients: HashMap<RawFd, (TcpStream, TcpStream)> = HashMap::new();
    let mut client_to_server: HashMap<RawFd, RawFd> = HashMap::new();
    let mut server_to_client: HashMap<RawFd, RawFd> = HashMap::new();

    let mut buffer = [0u8; 4096];

    loop {
        let mut read_fds: fd_set = unsafe { std::mem::zeroed() };
        let mut write_fds: fd_set = unsafe { std::mem::zeroed() };
        let mut max_fd = listen_fd;

        unsafe { FD_ZERO(&mut read_fds); FD_SET(listen_fd, &mut read_fds); }

        for (client_fd, (client, server)) in &clients {
            let client_fd = *client_fd;
            let server_fd = server.as_raw_fd();
            max_fd = max_fd.max(client_fd).max(server_fd);

            unsafe { FD_SET(client_fd, &mut read_fds); }
            unsafe { FD_SET(server_fd, &mut read_fds); }
        }

        let timeout = libc::timeval { tv_sec: 1, tv_usec: 0 };
        let result = unsafe {
            select(max_fd + 1, &mut read_fds, &mut write_fds, std::ptr::null_mut(), &timeout)
        };

        if result < 0 {
            break;
        }

        // Новое соединение
        if unsafe { FD_ISSET(listen_fd, &read_fds) } {
            if let Ok((client, _)) = listener.accept() {
                let client_fd = client.as_raw_fd();
                set_nonblocking(client_fd);

                match TcpStream::connect(format!("{}:{}", target_host, target_port)) {
                    Ok(server) => {
                        let server_fd = server.as_raw_fd();
                        set_nonblocking(server_fd);

                        client_to_server.insert(client_fd, server_fd);
                        server_to_client.insert(server_fd, client_fd);
                        clients.insert(client_fd, (client, server));
                    }
                    Err(_) => {
                        drop(client);
                    }
                }
            }
        }

        // Передача данных
        let mut to_remove = Vec::new();

        for (client_fd, (client, server)) in &mut clients {
            let client_fd = *client_fd;
            let server_fd = server.as_raw_fd();

            // Клиент -> Сервер
            if unsafe { FD_ISSET(client_fd, &read_fds) } {
                match client.read(&mut buffer) {
                    Ok(0) => { to_remove.push(client_fd); continue; }
                    Ok(n) => {
                        if server.write_all(&buffer[..n]).is_err() {
                            to_remove.push(client_fd);
                            continue;
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => { to_remove.push(client_fd); continue; }
                }
            }

            // Сервер -> Клиент
            if unsafe { FD_ISSET(server_fd, &read_fds) } {
                match server.read(&mut buffer) {
                    Ok(0) => { to_remove.push(client_fd); continue; }
                    Ok(n) => {
                        if client.write_all(&buffer[..n]).is_err() {
                            to_remove.push(client_fd);
                            continue;
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => { to_remove.push(client_fd); continue; }
                }
            }
        }

        for fd in to_remove {
            clients.remove(&fd);
            if let Some(srv) = client_to_server.remove(&fd) {
                server_to_client.remove(&srv);
            }
        }
    }
}
