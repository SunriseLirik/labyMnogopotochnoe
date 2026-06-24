use std::thread;

fn thread_func(strings: Vec<String>) {
    for s in strings {
        println!("Поток {:?}: {}", thread::current().id(), s);
    }
}

fn main() {
    let sequences = vec![
        vec!["Alpha".to_string(), "Bravo".to_string(), "Charlie".to_string()],
        vec!["One".to_string(), "Two".to_string(), "Three".to_string(), "Four".to_string()],
        vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
        vec!["First".to_string(), "Second".to_string(), "Third".to_string(), "Fourth".to_string(), "Fifth".to_string()],
    ];

    let mut handles = vec![];

    for seq in sequences {
        let handle = thread::spawn(move || {
            thread_func(seq);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
