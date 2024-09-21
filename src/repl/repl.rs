use crossterm::{
    self, cursor,
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    style,
    style::Stylize,
    terminal::{self, ClearType, EnableLineWrap},
    QueueableCommand,
};

use std::{
    collections::HashMap,
    error::Error,
    fmt,
    io::{self, Stdout, Write},
    result::Result,
    string::String,
    sync::atomic::{AtomicBool, Ordering},
    sync::Mutex,
    time::Duration,
};

use super::inputhistory::InputHistory;

/// CommandFunction is the callback that implements the actual behavior of the command
type CommandFunction<T> = dyn FnMut(Option<String>, &mut T) -> Result<String, String>;

/// Definition of a command that the REPL recognizes and executes
struct CommandDefinition<T> {
    /// Name of command, will be matched with the user input
    pub name: String,
    /// Take an argument, do stuff and return Messages to display
    pub function: Option<Box<CommandFunction<T>>>,
    /// Help message to be displayed after the `function` returns an Error object
    pub help: Option<String>,
}

/// REPL built-in commands, may not be overwritten
const BUILT_INS: [(&str, &str); 3] = [
    ("help", "Display help"),
    ("quit", "Terminate the application"),
    ("exit", "Terminate the application"),
];

/// Error that is given when you overwrite a built-in command
#[derive(Debug, Clone)]
pub struct BuiltInOverwriteError {
    cmd_name: String,
}
impl fmt::Display for BuiltInOverwriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "setting {} is not allowed because it is a built-in command",
            self.cmd_name
        )
    }
}
impl Error for BuiltInOverwriteError {}

/// Requirement for the object that the REPL interacts with
pub trait ReplApp {
    fn get_status(&self) -> String;
    fn get_event_interval(&self) -> Duration;
}

/// Implementation of a Read Print Evaluate Loop (REPL)
///
/// It has a status line that the underlying app needs to fill. It implements the default commands
/// 'quit' and 'help'. The REPL also catches CTRL+c and CTRL+d to exit the application.
pub struct Repl<T: ReplApp> {
    app: Mutex<T>,
    commands: HashMap<String, CommandDefinition<T>>,
    exit: AtomicBool,
    prompt: String,
    history: InputHistory,
}

impl<T> Repl<T>
where
    T: ReplApp,
{
    pub fn new(app: T, prompt: String) -> Self {
        let mut repl = Repl {
            app: Mutex::new(app),
            commands: HashMap::new(),
            exit: false.into(),
            prompt,
            history: InputHistory::new(),
        };
        for (cmd, help) in BUILT_INS {
            repl.commands.insert(
                cmd.to_string(),
                CommandDefinition {
                    name: cmd.to_string(),
                    function: None,
                    help: Some(help.to_string()),
                },
            );
        }
        repl
    }

    /// Add or update a command a REPL command
    ///
    /// A command is updated if `cmddef.command` matches a already added command
    pub fn set_command(
        &mut self,
        name: String,
        function: Box<CommandFunction<T>>,
        help: Option<String>,
    ) -> Result<(), BuiltInOverwriteError> {
        // check against built in commands
        let name_is_builtin = BUILT_INS
            .into_iter()
            .any(|built_in| built_in.0 == name.as_str());
        if name_is_builtin {
            return Err(BuiltInOverwriteError { cmd_name: name });
        }

        // add given command
        let mut cmd = CommandDefinition {
            name,
            function: Some(function),
            help,
        };
        // make sure that each help command ends with a new line
        if let Some(help_msg) = cmd.help {
            let append_newline = match help_msg.chars().last() {
                Some('\n') => false,
                None => false,
                Some(_) => true,
            };
            let mut new_help = help_msg;
            if append_newline {
                new_help.push('\n');
            }
            new_help = new_help.replace('\n', "\n\r");
            cmd.help = Some(new_help);
        }
        self.commands.insert(cmd.name.clone(), cmd);
        Ok(())
    }

    /// Start the REPL
    ///
    /// Waits for keyboard events to process them
    pub fn run(&mut self) -> io::Result<()> {
        self.exit.store(false, Ordering::SeqCst);
        let mut stdout = io::stdout();
        crossterm::terminal::enable_raw_mode()?;
        stdout.queue(EnableLineWrap {})?.flush()?;

        // print prompt first time
        self.refresh_prompt_status(&mut stdout, None)?;

        while !self.exit.load(Ordering::SeqCst) {
            if crossterm::event::poll(self.app.get_mut().unwrap().get_event_interval())? {
                if let Event::Key(event) = crossterm::event::read()? {
                    if event.modifiers == KeyModifiers::CONTROL
                        && (event.code == KeyCode::Char('c') || event.code == KeyCode::Char('d'))
                    {
                        break;
                    };
                    if event.kind != KeyEventKind::Release {
                        self.on_key_pressed(&mut stdout, &event.code)?;
                    }
                }
            }
        }
        // Exit, make sure to leave enough new lines so that the status line remain in command
        // window scroll back
        stdout
            .queue(terminal::ScrollUp(1))?
            .queue(cursor::MoveToNextLine(3))?
            .flush()?;
        Ok(())
    }

    /// React on key presses
    fn on_key_pressed(&mut self, stdout: &mut Stdout, key: &KeyCode) -> io::Result<()> {
        let mut key_message: Option<String> = None;
        let key_press_successful = match key {
            KeyCode::Char(c) => {
                self.history.add_char(c);
                true
            }
            KeyCode::Right => self.history.right(),
            KeyCode::Left => self.history.left(),
            KeyCode::Up => self.history.up(),
            KeyCode::Down => self.history.down(),
            KeyCode::Backspace => self.history.backspace(),
            KeyCode::Delete => self.history.del_key(),
            KeyCode::Enter => {
                let success = match self.parse_and_execute_command(self.history.get_line()) {
                    Ok(msg) => {
                        key_message = Some(msg);
                        true
                    }
                    Err(msg) => {
                        key_message = Some(msg);
                        false
                    }
                };
                stdout.queue(terminal::ScrollUp(1))?;
                self.history.add_line();
                success
            }
            _ => false,
        };

        // Message that needs to be displayed
        let output_msg = if let Some(msg) = key_message {
            let mut output_msg = msg;
            if !key_press_successful {
                output_msg.insert_str(0, "Error: ");
            }
            Some(output_msg)
        } else {
            None
        };

        self.refresh_prompt_status(stdout, output_msg)
    }

    /// Refresh prompt and status line
    fn refresh_prompt_status(
        &mut self,
        stdout: &mut Stdout,
        output_msg: Option<String>,
    ) -> io::Result<()> {
        let (_, rows) = terminal::size()?;

        let prompt = &self.prompt;

        // display output message
        if let Some(msg) = output_msg {
            stdout
                .queue(terminal::Clear(ClearType::CurrentLine))?
                .queue(cursor::MoveToColumn(0))?
                .queue(style::Print(msg))?
                .queue(terminal::ScrollUp(1))?
                .queue(cursor::MoveToNextLine(1))?;
        }

        // refresh prompt and status line
        stdout
            // print status line
            .queue(cursor::MoveTo(0, rows))?
            .queue(terminal::Clear(ClearType::CurrentLine))?
            .queue(style::Print(
                self.app.get_mut().unwrap().get_status().negative(),
            ))?
            // print prompt
            .queue(cursor::MoveUp(1))?
            .queue(terminal::Clear(ClearType::CurrentLine))?
            .queue(cursor::MoveToColumn(0))?
            .queue(style::Print(prompt))?
            .queue(style::Print(self.history.get_line()))?
            .queue(cursor::MoveToColumn(
                (prompt.chars().count() + self.history.column()) as u16,
            ))?;

        // make output happen
        stdout.flush()?;
        Ok(())
    }

    /// Match input with known commands and react appropriately
    fn parse_and_execute_command(&mut self, input: String) -> Result<String, String> {
        // remove every white space from left, iterate over the lines, take only the first line
        let (parsed_cmd, args) = parse_cmd_w_args(input);

        // match predefined commands
        match parsed_cmd.as_str() {
            "quit" | "exit" => {
                self.exit.store(true, Ordering::SeqCst);
                return Ok(String::from("Exiting"));
            }
            "help" => {
                // show all commands no argument is given
                if args.is_empty() {
                    return Ok(format!(
                        "Known commands: {}\n{}",
                        self.list_commands(),
                        "Use \"help <COMMAND>\" to get the help message for the command if \
                            available",
                    ));
                }
                // show help for command given as argument
                else {
                    match self.commands.get_mut(args.as_str()) {
                        Some(cmddef) => {
                            if let Some(help_msg) = &cmddef.help {
                                return Ok(help_msg.clone());
                            } else {
                                return Ok(String::from("No help message"));
                            }
                        }
                        None => return Err(format!("Command \"{}\" is unknown!", args)),
                    }
                }
            }
            _ => (),
        }

        // match custom commands
        match self.commands.get_mut(parsed_cmd.as_str()) {
            Some(cmddef) => {
                let cmd_result = if cmddef.function.is_some() {
                    if !args.is_empty() {
                        (cmddef.function.as_mut().unwrap())(Some(args), self.app.get_mut().unwrap())
                    } else {
                        (cmddef.function.as_mut().unwrap())(None, self.app.get_mut().unwrap())
                    }
                } else {
                    Err("No function associated".to_string())
                };
                match cmd_result {
                    Ok(msg) => Ok(msg),
                    Err(err_msg) => {
                        let mut msg = format!("Error in command \"{}\": {}", cmddef.name, err_msg);
                        if let Some(help_msg) = &cmddef.help {
                            msg += format!(" Command usage: {}", help_msg).as_ref();
                        }
                        Err(msg)
                    }
                }
            }
            None => {
                let msg = format!(
                    "\"{}\" command unknown! Known commands: {}",
                    parsed_cmd,
                    self.list_commands()
                );
                Err(msg)
            }
        }
    }

    fn list_commands(&self) -> String {
        let mut commands = String::new();
        for (cmd, cmddef) in self.commands.iter() {
            if cmd.is_empty() {
                commands += "<ENTER> ";
            } else {
                commands += format!("\"{}\" ", cmddef.name).as_ref();
            }
        }
        if !commands.is_empty() {
            commands.remove(commands.len() - 1);
        }
        commands
    }
}

/// Parse command and arguments from input
///
/// Splits the input string into the first word (command) and the rest of the string (arguments)
fn parse_cmd_w_args(input: String) -> (String, String) {
    let (command_str, args_str) = if input.is_empty() {
        (String::from(""), String::from(""))
    } else {
        let trimmed_input = input.trim_start().lines().next().unwrap_or("");
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
