use std::io;

pub struct Repl {
    pub commands: Vec<(String, fn(&Option<&str>))>,
    pub exit: bool,
}

impl Repl {
    pub fn process(&mut self) {
        self.exit = false;
        while !self.exit {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let mut splitted = input.trim().split_whitespace();
                    let mut parsed_cmd = splitted.next();
                    match parsed_cmd {
                        Some("quit") | Some("exit") => {
                            self.exit = true;
                            continue;
                        }
                        Some(&_) => (),
                        None => parsed_cmd = Some(""),
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
