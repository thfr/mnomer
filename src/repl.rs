use std::io::{self, Write};
use std::string::String;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub struct Repl<T> {
    pub app: Mutex<T>,
    pub commands: Vec<(String, Box<dyn FnMut(Option<&str>, &mut T)>)>,
    pub exit: AtomicBool,
    pub prompt: String,
}

impl<T> Repl<T> {
    /// Make the REPL go until self.exit is set to true
    pub fn process(&mut self) {
        let mut app = self.app.lock().unwrap();
        // TODO: make this function testable by splitting it
        //       maybe use some kind of buffers so that std::std{in,out} may be exchanged for testing
        self.exit.store(false, Ordering::SeqCst);
        let mut io_out = io::stdout();
        let io_in = io::stdin();
        while !self.exit.load(Ordering::SeqCst) {
            io_out.write_all(self.prompt.as_ref()).unwrap();
            io_out.flush().unwrap();
            let mut input = String::new();
            match io_in.read_line(&mut input) {
                Ok(nchars) => {
                    // remove every whitespace from left, iterate over the lines, take only the first line
                    let (parsed_cmd, args) = if nchars == 0 {
                        ("", "")
                    } else {
                        let trimmed_input = match input.trim_start().lines().next() {
                            Some(string) => string,
                            None => "",
                        };
                        match trimmed_input.find(char::is_whitespace) {
                            Some(pos) => (
                                &trimmed_input[0..pos],
                                trimmed_input[pos + 1..].trim_start(),
                            ),
                            None => (trimmed_input, ""),
                        }
                    };
                    match parsed_cmd {
                        "quit" | "exit" => {
                            self.exit.store(true, Ordering::SeqCst);
                            continue;
                        }
                        _ => (),
                    }
                    // check if parsed command is in self.commands and execute its function
                    for (cmd, function) in &mut self.commands {
                        if parsed_cmd == cmd {
                            if !args.is_empty() {
                                function(Some(args), &mut app);
                            } else {
                                function(None, &mut app);
                            }
                            break;
                        };
                    }
                }
                Err(error) => println!("error: {}", error),
            }
        }
    }
}
