use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => (),
            Err(error) => panic!("Failed to read line: {:?}", error),
        };

        let mut list = match shell_in_rust::parse(&input) {
            Ok(l) => l,
            Err(s) => {
                eprintln!("{}", s);
                continue;
            }
        };

        if let Err(e) = shell_in_rust::invoke_cmd(&mut list, true) {
            eprintln!("Command failed: {}", input.trim());
            if cfg!(debug_assertions) {
                eprintln!("{}", e);
            }
            continue;
        }

        if let Err(e) = shell_in_rust::wait_cmdline(&mut list) {
            eprintln!("{}", e);
        }
    }
}
