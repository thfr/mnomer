use crossterm::{
    self, cursor,
    event::{Event, KeyCode, KeyModifiers},
    style,
    terminal::{self, ClearType},
    QueueableCommand,
};

use std::string::String;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::{
    collections::HashMap,
    io::{self, Stdout, Write},
    iter::FromIterator,
};

pub struct CommandDefinition<T> {
    pub command: String,
    pub function: Box<dyn FnMut(Option<String>, &mut T) -> Result<String, String>>,
    pub help: Option<String>,
}

pub struct Repl<T> {
    pub app: Mutex<T>,
    pub commands: HashMap<String, CommandDefinition<T>>,
    pub exit: AtomicBool,
    pub prompt: String,
    pub history: InputHistory,
}

impl<T> Repl<T> {
    /// Add or update a command a REPL command
    ///
    /// A command is updated if `cmddef.command` matches a already added command
    pub fn set_command(&mut self, cmddef: CommandDefinition<T>) {
        self.commands.insert(cmddef.command.clone(), cmddef);
    }

    pub fn run_with_crossterm(&mut self) -> crossterm::Result<()> {
        self.exit.store(false, Ordering::SeqCst);
        let mut stdout = io::stdout();
        crossterm::terminal::enable_raw_mode()?;

        stdout.queue(style::Print(&self.prompt))?.flush()?;
        while !self.exit.load(Ordering::SeqCst) {
            match crossterm::event::read()? {
                Event::Key(event) => {
                    if event.code == KeyCode::Char('c') && event.modifiers == KeyModifiers::CONTROL
                    {
                        break;
                    };
                    self.on_key_pressed(&mut stdout, &event.code)?;
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn on_key_pressed(&mut self, stdout: &mut Stdout, key: &KeyCode) -> crossterm::Result<()> {
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
                        stdout
                            .queue(terminal::ScrollUp(1))?
                            .queue(cursor::MoveToNextLine(1))?;
                        self.history.add_line();
                        key_message = Some(msg);
                        true
                    }
                    Err(msg) => {
                        key_message = Some(msg);
                        false
                    }
                };
                success
            }
            _ => false,
        };
        let prompt = &self.prompt;
        let output_msg = if let Some(msg) = key_message {
            let mut output_msg = msg;
            if !key_press_successful {
                output_msg.insert_str(0, "Error: ");
            }
            output_msg
        } else {
            String::new()
        };
        let (_, cursor_row) = cursor::position()?;
        let (_, rows) = terminal::size()?;
        if cursor_row + 1 == rows {
            // last line, we add a new line but scrolling
            stdout.queue(terminal::ScrollUp(1))?;
        }
        stdout
            .queue(cursor::MoveToNextLine(1))?
            .queue(terminal::Clear(ClearType::CurrentLine))?
            .queue(style::Print(output_msg))?
            .queue(cursor::MoveUp(1))?
            .queue(terminal::Clear(ClearType::CurrentLine))?
            .queue(cursor::MoveToColumn(0))?
            .queue(style::Print(prompt))?
            .queue(style::Print(self.history.get_line()))?
            .queue(cursor::MoveToColumn(
                (prompt.chars().count() + self.history.column + 1) as u16,
            ))?
            .flush()?;
        Ok(())
    }

    /// Give an error message to display
    fn parse_and_execute_command(&mut self, input: String) -> Result<String, String> {
        // remove every whitespace from left, iterate over the lines, take only the first line
        let (parsed_cmd, args) = parse_cmd_w_args(input);

        match parsed_cmd.as_str() {
            "quit" | "exit" => {
                self.exit.store(true, Ordering::SeqCst);
                return Ok(String::from("Exiting"));
            }
            "help" => match self.commands.get_mut(args.as_str()) {
                Some(cmddef) => {
                    if let Some(help_msg) = &cmddef.help {
                        return Ok(help_msg.clone());
                    } else {
                        return Ok(String::from("No help message"));
                    }
                }
                None => {
                    let msg = format!(
                        "\"{}\" command unknown! {}",
                        parsed_cmd,
                        self.list_commands()
                    );
                    return Err(msg);
                }
            },
            _ => (),
        }
        // check if parsed command is in self.commands and execute its function
        match self.commands.get_mut(parsed_cmd.as_str()) {
            Some(cmddef) => {
                let cmd_result = if !args.is_empty() {
                    (cmddef.function)(Some(args), self.app.get_mut().unwrap())
                } else {
                    (cmddef.function)(None, self.app.get_mut().unwrap())
                };
                match cmd_result {
                    Ok(msg) => Ok(msg),
                    Err(err_msg) => {
                        let mut msg =
                            format!("Error in command \"{}\": {}", cmddef.command, err_msg);
                        if let Some(help_msg) = &cmddef.help {
                            msg += format!(" Command usage: {}", help_msg).as_ref();
                        }
                        Err(msg)
                    }
                }
            }
            None => {
                let msg = format!(
                    "\"{}\" command unknown! {}",
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
                commands += format!("<ENTER> ").as_ref();
            } else {
                commands += format!("\"{}\" ", cmddef.command).as_ref();
            }
        }
        if commands.len() > 0 {
            commands.remove(commands.len() - 1);
        }
        commands
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

/// Represent command history
pub struct InputHistory {
    /// Previous inputs, should not be altered
    previous_lines: Vec<Vec<char>>,
    /// Current input, which is altered
    writing_buffer: Vec<char>,
    /// If row equals length of previous_lines, then display `writing_buffer`, else display a line from `previous_lines`
    row: usize,
    /// Cursor column so that we know where to put in the character
    column: usize,
}

impl InputHistory {
    pub fn new() -> InputHistory {
        InputHistory {
            // initialize with `previous_lines.len() == 0`
            previous_lines: vec![],
            writing_buffer: vec![],
            row: 0,
            column: 0,
        }
    }

    fn _row_in_previous_lines(&self) -> bool {
        self.row < self.previous_lines.len() && self.previous_lines.len() > 0
    }

    fn _prepare_modifying_access(&mut self) {
        if self._row_in_previous_lines() {
            self.writing_buffer
                .clone_from(&self.previous_lines[self.row]);
            self.row = self.previous_lines.len();
        }
    }

    fn _current_line_len(&self) -> usize {
        if self.row == self.previous_lines.len() {
            self.writing_buffer.len()
        } else {
            self.previous_lines[self.row].len()
        }
    }

    fn add_char(&mut self, c: &char) {
        self._prepare_modifying_access();
        self.writing_buffer.insert(self.column, *c);
        self.column += 1;
    }

    fn delete_char(&mut self) -> bool {
        self._prepare_modifying_access();
        if self.column < self.writing_buffer.len() {
            self.writing_buffer.remove(self.column);
            true
        } else {
            false
        }
    }

    fn add_line(&mut self) -> bool {
        self._prepare_modifying_access();
        let current_line = std::mem::replace(&mut self.writing_buffer, vec![]);
        self.previous_lines.push(current_line);
        self.row = self.previous_lines.len();
        self.column = 0;
        true
    }

    fn get_line(&self) -> String {
        if self._row_in_previous_lines() {
            String::from_iter(self.previous_lines[self.row].iter())
        } else {
            String::from_iter(self.writing_buffer.iter())
        }
    }

    #[allow(dead_code)]
    fn debug_status(&self) -> String {
        format!("R={:3} C={:3}: ", self.row, self.column)
    }

    ////////////////////////////
    // Keystroke implementations
    ////////////////////////////

    fn right(&mut self) -> bool {
        if self.column < self._current_line_len() {
            self.column += 1;
            true
        } else {
            false
        }
    }

    fn left(&mut self) -> bool {
        if self.column != 0 {
            self.column -= 1;
            true
        } else {
            false
        }
    }

    fn down(&mut self) -> bool {
        if self.row < self.previous_lines.len() {
            self.row += 1;
            self.column = self._current_line_len();
            true
        } else {
            false
        }
    }

    fn up(&mut self) -> bool {
        if self.row != 0 {
            self.row -= 1;
            self.column = self.previous_lines[self.row].len();
            true
        } else {
            false
        }
    }

    fn backspace(&mut self) -> bool {
        if self.column > 0 {
            self.column -= 1;
            self.delete_char()
        } else {
            false
        }
    }

    fn del_key(&mut self) -> bool {
        self.delete_char()
    }
}
