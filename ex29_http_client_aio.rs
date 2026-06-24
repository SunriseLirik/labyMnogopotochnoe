use std::io::{self, Write};
use std::net::TcpStream;
use libc::{aio_read, aio_write, aiocb, sigevent, off_t, ssize_t, EINPROGRESS};
use std::os::unix::io::AsRawFd;

const BUFFER_SIZE: usize = 4096;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <url>", args[0]);
        std::process::exit(1);
    }

    // Парсинг URL (упрощённый)
    let url = args[1].strip_prefix("http://").unwrap_or(&args[1]);
    let parts: Vec<&str> = url.splitn(2, '/').collect();
    let host = parts[0].split(':').next().unwrap();
    let path = if parts.len() > 1 { format!("/{}", parts[1]) } else { "/".to_string() };

    let mut stream = TcpStream::connect(format!("{}:80", host)).unwrap();
    let request = format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", path, host);
    stream.write_all(request.as_bytes()).unwrap();

    let fd = stream.as_raw_fd();

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut aio = aiocb {
        aio_fildes: fd,
        aio_lio_opcode: 0,
        aio_reqprio: 0,
        aio_buf: buffer.as_mut_ptr() as *mut _,
        aio_nbytes: BUFFER_SIZE,
        aio_offset: 0,
        aio_sigevent: sigevent {
            sigev_value: unsafe { std::mem::zeroed() },
            sigev_signo: 0,
            sigev_notify: 0,
        },
        __next_prio: std::ptr::null_mut(),
        __abs_prio: 0,
        __policy: 0,
        __error_code: 0,
        __return_value: 0,
    };

    let mut header_skipped = false;
    let mut lines_printed = 0;
    let mut waiting_for_space = false;

    loop {
        unsafe { aio_read(&mut aio); }

        // Ждём завершения aio
        let mut err = 0;
        while err == EINPROGRESS || err == 0 {
            err = unsafe { libc::aio_error(&aio) };
            if err == EINPROGRESS {
                // Проверяем ввод пользователя
                if waiting_for_space {
                    check_user_input(&mut waiting_for_space, &mut lines_printed);
                }
            }
        }

        let n = unsafe { libc::aio_return(&aio) };
        if n <= 0 {
            break;
        }

        if !header_skipped {
            let data = String::from_utf8_lossy(&buffer[..n as usize]);
            if let Some(pos) = data.find("\r\n\r\n") {
                let body_start = pos + 4;
                process_output(&buffer[body_start..n as usize], &mut lines_printed, &mut waiting_for_space);
                header_skipped = true;
            }
        } else {
            process_output(&buffer[..n as usize], &mut lines_printed, &mut waiting_for_space);
        }

        // Обновляем offset для следующего чтения
        aio.aio_offset += n as off_t;
    }
}

fn process_output(data: &[u8], lines_printed: &mut usize, waiting: &mut bool) {
    let text = String::from_utf8_lossy(data);
    for line in text.split('\n') {
        if *waiting {
            return;
        }
        if *lines_printed >= 25 {
            *waiting = true;
            println!("\n-- Press space to scroll down --");
            return;
        }
        println!("{}", line);
        *lines_printed += 1;
    }
}

fn check_user_input(waiting: &mut bool, lines_printed: &mut usize) {
    use std::io::Read;
    let mut buf = [0u8; 1];
    if io::stdin().read(&mut buf).unwrap_or(0) > 0 && buf[0] == b' ' {
        *waiting = false;
        *lines_printed = 0;
    }
}
