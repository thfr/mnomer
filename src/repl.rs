use std::io::{self, Write};
use std::string::String;

pub struct Repl {
    pub commands: Vec<(String, Box<dyn FnMut(Option<&str>)>)>,
    pub exit: bool,
    pub prompt: String,
}

impl Repl {
    pub fn process(&mut self) {
        self.exit = false;
        let mut io_out = io::stdout();
        let io_in = io::stdin();
        while !self.exit {
            io_out.write_all(self.prompt.as_ref()).unwrap();
            io_out.flush().unwrap();
            let mut input = String::new();
            match io_in.read_line(&mut input) {
                Ok(_) => {
                    let mut splitted = input.trim().split_whitespace();
                    let mut parsed_cmd = splitted.next();
                    match parsed_cmd {
                        Some("quit") | Some("exit") => {
                            self.exit = true;
                            continue;
                        }
                        // let other command be matched against self.commands
                        Some(&_) => (),
                        // convert None to empty string so that a single enter may be parsed
                        None => parsed_cmd = Some(""),
                    }
                    // check if parsed command is in self.commands and execute its function
                    for (cmd, function) in &mut self.commands {
                        match parsed_cmd {
                            Some(parsed_cmd) => {
                                if parsed_cmd == cmd {
                                    let args = splitted.next();
                                    function(args);
                                    break;
                                };
                            }
                            None => (),
                        };
                    }
                }
                Err(error) => println!("error: {}", error),
            }
        }
    }
}
