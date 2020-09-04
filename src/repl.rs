use std::io;

pub struct Repl {
    pub commands: Vec<(String, fn(&Option<&str>))>,
    pub exit: bool,
}

impl Repl {
    fn process(&mut self) {
        while !self.exit {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(n) => {
                    println!("{} bytes read", n);
                    println!("{}", input);
                    let mut splitted = input.trim().split_whitespace();
                    let parsed_cmd = splitted.next();
                    match parsed_cmd {
                        Some("quit") | Some("exit") => {
                            self.exit = true;
                            continue;
                        }
                        Some(&_) => (),
                        None => continue,
                    }
                    for (cmd, function) in &self.commands {
                        let args = splitted.next();
                        match parsed_cmd {
                            Some(parsed_cmd) => {
                                if parsed_cmd == cmd {
                                    function(&args)
                                }
                            }
                            None => (),
                        }
                    }
                }
                Err(error) => println!("error: {}", error),
            }
        }
    }
}
