use std::io::{self, Write};

fn main() {
    let mut command = String::new();

    print!("$ ");
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut command)
        .expect("Failed to read line");
}
