/// Represent command history
///
/// Implements a virtual cursor (row, column) and provides keystroke implementations for cursor navigation
#[derive(Debug, PartialEq)]
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

    pub fn row(&self) -> usize {
        return self.row;
    }

    pub fn column(&self) -> usize {
        return self.column;
    }

    fn _row_in_previous_lines(&self) -> bool {
        self.row < self.previous_lines.len() && !self.previous_lines.is_empty()
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

    pub fn add_char(&mut self, c: &char) {
        self._prepare_modifying_access();
        self.writing_buffer.insert(self.column, *c);
        self.column += 1;
    }

    pub fn delete_char(&mut self) -> bool {
        self._prepare_modifying_access();
        if self.column < self.writing_buffer.len() {
            self.writing_buffer.remove(self.column);
            true
        } else {
            false
        }
    }

    pub fn add_line(&mut self) -> bool {
        self._prepare_modifying_access();
        let current_line = std::mem::take(&mut self.writing_buffer);
        self.previous_lines.push(current_line);
        self.row = self.previous_lines.len();
        self.column = 0;
        true
    }

    pub fn get_line(&self) -> String {
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

    pub fn right(&mut self) -> bool {
        if self.column < self._current_line_len() {
            self.column += 1;
            true
        } else {
            false
        }
    }

    pub fn left(&mut self) -> bool {
        if self.column != 0 {
            self.column -= 1;
            true
        } else {
            false
        }
    }

    pub fn down(&mut self) -> bool {
        if self.row < self.previous_lines.len() {
            self.row += 1;
            self.column = self._current_line_len();
            true
        } else {
            false
        }
    }

    pub fn up(&mut self) -> bool {
        if self.row != 0 {
            self.row -= 1;
            self.column = self.previous_lines[self.row].len();
            true
        } else {
            false
        }
    }

    pub fn backspace(&mut self) -> bool {
        if self.column > 0 {
            self.column -= 1;
            self.delete_char()
        } else {
            false
        }
    }

    pub fn del_key(&mut self) -> bool {
        self.delete_char()
    }
}

#[cfg(test)]
mod test_inputhistory {
    use super::*;

    #[test]
    fn test_backspace() {
        let mut history_test = InputHistory::new();
        let mut history_compare = InputHistory::new();
        assert_eq!(history_test, history_compare);
        assert_eq!(history_test.column(), history_compare.column());
        assert_eq!(history_test.row(), history_compare.row());

        history_test.add_char(&'c');
        history_compare.add_char(&'c');
        history_test.backspace();

        assert!(history_compare.writing_buffer.pop().is_some());
        history_compare.column -= 1;
        assert_eq!(history_test, history_compare);
        assert_eq!(history_test.column(), history_compare.column());
        assert_eq!(history_test.row(), history_compare.row());
    }

    #[test]
    fn test_add_char() {
        let mut history_test = InputHistory::new();
        let mut history_compare = InputHistory::new();
        assert_eq!(history_test, history_compare);
        assert_eq!(history_test.column(), history_compare.column());
        assert_eq!(history_test.row(), history_compare.row());

        history_test.add_char(&'c');

        history_compare.writing_buffer.push('c');
        history_compare.column += 1;
        assert_eq!(history_test, history_compare);
        assert_eq!(history_test.column(), history_compare.column());
        assert_eq!(history_test.row(), history_compare.row());
    }

    #[test]
    fn test_add_line() {
        let mut history_test = InputHistory::new();
        let mut history_compare = InputHistory::new();
        assert_eq!(history_test, history_compare);
        assert_eq!(history_test.column(), history_compare.column());
        assert_eq!(history_test.row(), history_compare.row());

        history_test.add_line();

        history_compare.row += 1;
        history_compare.previous_lines.push(vec![]);
        assert_eq!(history_test, history_compare);
        assert_eq!(history_test.column(), history_compare.column());
        assert_eq!(history_test.row(), history_compare.row());
    }
}
