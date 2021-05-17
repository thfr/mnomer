use std::string::String;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::{
    collections::HashMap,
    io::{self, Write},
};

pub struct CommandDefinition<T> {
    pub command: String,
    pub function: Box<dyn FnMut(Option<String>, &mut T) -> Result<(), String>>,
    pub help: Option<String>,
}

pub struct Repl<T> {
    pub app: Mutex<T>,
    pub commands: HashMap<String, CommandDefinition<T>>,
    pub exit: AtomicBool,
    pub prompt: String,
}

impl<T> Repl<T> {
    /// Add or update a command a REPL command
    ///
    /// A command is updated if `cmddef.command` matches a already added command
    pub fn set_command(&mut self, cmddef: CommandDefinition<T>) {
        self.commands.insert(cmddef.command.clone(), cmddef);
    }

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
                Ok(_) => {
                    // remove every whitespace from left, iterate over the lines, take only the first line
                    let (parsed_cmd, args) = parse_cmd_w_args(input);

                    match parsed_cmd.as_str() {
                        "quit" | "exit" => {
                            self.exit.store(true, Ordering::SeqCst);
                            continue;
                        }
                        _ => (),
                    }
                    // check if parsed command is in self.commands and execute its function
                    match self.commands.get_mut(parsed_cmd.as_str()) {
                        Some(cmddef) => {
                            let cmd_result = if !args.is_empty() {
                                (cmddef.function)(Some(args), &mut app)
                            } else {
                                (cmddef.function)(None, &mut app)
                            };
                            match cmd_result {
                                Err(err_msg) => {
                                    println!(
                                        "Error in command \"{}\": {}",
                                        cmddef.command, err_msg
                                    );
                                    if cmddef.help.is_some() {
                                        println!(
                                            "Command usage: {}",
                                            cmddef.help.as_ref().unwrap()
                                        );
                                        println!("");
                                    }
                                }
                                Ok(_) => (),
                            };
                        }
                        None => {
                            println!("\"{}\" command unknown", parsed_cmd);
                            self.print_commmands();
                        }
                    };
                }
                Err(error) => println!("error: {}", error),
            }
        }
    }

    fn print_commmands(&self) {
        println!("Following commands are defined:");
        for (cmd, cmddef) in self.commands.iter() {
            if cmd.is_empty() {
                println!("<ENTER>");
            } else {
                println!("\"{}\"", cmddef.command);
            }
        }
        println!("");
    }
}

/// Parse command and arguments from input
///
/// Splits the input string into a the first word (command) and the rest of the string (arguments)
fn parse_cmd_w_args(input: String) -> (String, String) {
    let (command_str, args_str) = if input.len() == 0 {
        (String::from(""), String::from(""))
    } else {
        let trimmed_input = match input.trim_start().lines().next() {
            Some(string) => string,
            None => "",
        };
        match trimmed_input.find(char::is_whitespace) {
            Some(pos) => (
                String::from(&trimmed_input[0..pos]),
                String::from(trimmed_input[pos + 1..].trim_start()),
            ),
            None => (String::from(trimmed_input), String::from("")),
        }
    };
    (command_str, args_str)
}
