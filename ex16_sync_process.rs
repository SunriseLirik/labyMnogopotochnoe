use libc::{sem_close, sem_open, sem_post, sem_wait, SEM_FAILED};
use std::ffi::CString;
use std::process::{Command, Stdio};

const SEM_PARENT: &str = "/sem_parent_11";
const SEM_CHILD: &str = "/sem_child_11";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        // Родительский процесс: создаём семафоры и запускаем дочерний
        unsafe {
            // Удаляем старые семафоры, если есть
            libc::sem_unlink(CString::new(SEM_PARENT).unwrap().as_ptr());
            libc::sem_unlink(CString::new(SEM_CHILD).unwrap().as_ptr());

            // Создаём семафоры
            let sem_parent = sem_open(
                CString::new(SEM_PARENT).unwrap().as_ptr(),
                libc::O_CREAT,
                0o666,
                1,
            );
            let sem_child = sem_open(
                CString::new(SEM_CHILD).unwrap().as_ptr(),
                libc::O_CREAT,
                0o666,
                0,
            );

            // Запускаем дочерний процесс (этот же бинарник с аргументом)
            let _child = Command::new(&args[0])
                .arg("child")
                .stdout(Stdio::inherit())
                .spawn()
                .unwrap();

            // Родительский вывод
            for i in 1..=10 {
                sem_wait(sem_parent);
                println!("Родительский процесс: строка {}", i);
                sem_post(sem_child);
            }

            sem_close(sem_parent);
            sem_close(sem_child);
        }
    } else {
        // Дочерний процесс
        unsafe {
            let sem_parent = sem_open(
                CString::new(SEM_PARENT).unwrap().as_ptr(),
                0,
            );
            let sem_child = sem_open(
                CString::new(SEM_CHILD).unwrap().as_ptr(),
                0,
            );

            for i in 1..=10 {
                sem_wait(sem_child);
                println!("Дочерний процесс: строка {}", i);
                sem_post(sem_parent);
            }

            sem_close(sem_parent);
            sem_close(sem_child);
        }
    }
}
