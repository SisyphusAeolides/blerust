use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    InsertChar(usize, char),
    InsertStr(usize, String),
    DeleteChar(usize, char),
    DeleteStr(usize, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineBuffer {
    buffer: Vec<char>,
    cursor: usize,
    kill_ring: Vec<String>,
    undo_stack: Vec<Vec<EditAction>>,
    redo_stack: Vec<Vec<EditAction>>,
    current_transaction: Vec<EditAction>,
    is_inserting: bool,
}

impl Default for LineBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LineBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            cursor: 0,
            kill_ring: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_transaction: Vec::new(),
            is_inserting: false,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let buffer: Vec<char> = text.chars().collect();
        let cursor = buffer.len();
        Self {
            buffer,
            cursor,
            kill_ring: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_transaction: Vec::new(),
            is_inserting: false,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Self {
        Self::from_text(text)
    }

    pub fn as_str(&self) -> String {
        self.buffer.iter().collect()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.buffer.len());
        self.commit_transaction();
    }

    pub fn chars(&self) -> &[char] {
        &self.buffer
    }

    fn commit_transaction(&mut self) {
        if !self.current_transaction.is_empty() {
            self.undo_stack.push(self.current_transaction.clone());
            self.current_transaction.clear();
            self.redo_stack.clear();
        }
        self.is_inserting = false;
    }

    pub fn insert_char(&mut self, ch: char) {
        if !self.is_inserting {
            self.commit_transaction();
            self.is_inserting = true;
        }
        self.buffer.insert(self.cursor, ch);
        self.current_transaction
            .push(EditAction::InsertChar(self.cursor, ch));
        self.cursor += 1;
    }

    pub fn insert_str(&mut self, text: &str) {
        self.commit_transaction();
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return;
        }

        let mut inserted_str = String::new();
        for ch in chars {
            self.buffer.insert(self.cursor, ch);
            inserted_str.push(ch);
            self.cursor += 1;
        }
        self.undo_stack.push(vec![EditAction::InsertStr(
            self.cursor - inserted_str.chars().count(),
            inserted_str,
        )]);
        self.redo_stack.clear();
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor > 0 {
            if self.is_inserting {
                self.commit_transaction();
            }
            self.cursor -= 1;
            let ch = self.buffer.remove(self.cursor);
            self.current_transaction
                .push(EditAction::DeleteChar(self.cursor, ch));
            true
        } else {
            false
        }
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor < self.buffer.len() {
            if self.is_inserting {
                self.commit_transaction();
            }
            let ch = self.buffer.remove(self.cursor);
            self.current_transaction
                .push(EditAction::DeleteChar(self.cursor, ch));
            true
        } else {
            false
        }
    }

    pub fn move_cursor_left(&mut self) -> bool {
        self.commit_transaction();
        if self.cursor > 0 {
            self.cursor -= 1;
            true
        } else {
            false
        }
    }

    pub fn move_cursor_right(&mut self) -> bool {
        self.commit_transaction();
        if self.cursor < self.buffer.len() {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.commit_transaction();
        self.cursor = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.commit_transaction();
        self.cursor = self.buffer.len();
    }

    pub fn move_word_left(&mut self) {
        self.commit_transaction();
        if self.cursor == 0 {
            return;
        }
        let mut idx = self.cursor;
        while idx > 0 && self.buffer[idx - 1].is_whitespace() {
            idx -= 1;
        }
        while idx > 0 && !self.buffer[idx - 1].is_whitespace() {
            idx -= 1;
        }
        self.cursor = idx;
    }

    pub fn move_word_right(&mut self) {
        self.commit_transaction();
        let len = self.buffer.len();
        if self.cursor >= len {
            return;
        }
        let mut idx = self.cursor;
        while idx < len && !self.buffer[idx].is_whitespace() {
            idx += 1;
        }
        while idx < len && self.buffer[idx].is_whitespace() {
            idx += 1;
        }
        self.cursor = idx;
    }

    pub fn kill_to_end(&mut self) {
        self.commit_transaction();
        if self.cursor < self.buffer.len() {
            let killed: String = self.buffer.drain(self.cursor..).collect();
            self.undo_stack
                .push(vec![EditAction::DeleteStr(self.cursor, killed.clone())]);
            self.redo_stack.clear();
            self.kill_ring.push(killed);
        }
    }

    pub fn kill_to_start(&mut self) {
        self.commit_transaction();
        if self.cursor > 0 {
            let killed: String = self.buffer.drain(0..self.cursor).collect();
            self.undo_stack
                .push(vec![EditAction::DeleteStr(0, killed.clone())]);
            self.redo_stack.clear();
            self.cursor = 0;
            self.kill_ring.push(killed);
        }
    }

    pub fn kill_word_left(&mut self) {
        self.commit_transaction();
        if self.cursor == 0 {
            return;
        }
        let old_cursor = self.cursor;
        self.move_word_left();
        let new_cursor = self.cursor;
        let killed: String = self.buffer.drain(new_cursor..old_cursor).collect();
        self.undo_stack
            .push(vec![EditAction::DeleteStr(new_cursor, killed.clone())]);
        self.redo_stack.clear();
        self.kill_ring.push(killed);
    }

    pub fn yank(&mut self) {
        self.commit_transaction();
        if let Some(text) = self.kill_ring.last() {
            let yank_str = text.clone();
            self.insert_str(&yank_str);
        }
    }

    pub fn undo(&mut self) -> bool {
        self.commit_transaction();
        if let Some(mut actions) = self.undo_stack.pop() {
            let mut redo_actions = Vec::new();
            actions.reverse(); // Apply undo in reverse order
            for action in actions {
                match action {
                    EditAction::InsertChar(pos, ch) => {
                        self.buffer.remove(pos);
                        self.cursor = pos;
                        redo_actions.push(EditAction::DeleteChar(pos, ch));
                    }
                    EditAction::InsertStr(pos, text) => {
                        let len = text.chars().count();
                        self.buffer.drain(pos..pos + len);
                        self.cursor = pos;
                        redo_actions.push(EditAction::DeleteStr(pos, text.clone()));
                    }
                    EditAction::DeleteChar(pos, ch) => {
                        self.buffer.insert(pos, ch);
                        self.cursor = pos + 1;
                        redo_actions.push(EditAction::InsertChar(pos, ch));
                    }
                    EditAction::DeleteStr(pos, text) => {
                        let chars: Vec<char> = text.chars().collect();
                        let len = chars.len();
                        for (i, ch) in chars.into_iter().enumerate() {
                            self.buffer.insert(pos + i, ch);
                        }
                        self.cursor = pos + len;
                        redo_actions.push(EditAction::InsertStr(pos, text.clone()));
                    }
                }
            }
            redo_actions.reverse();
            self.redo_stack.push(redo_actions);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        self.commit_transaction();
        if let Some(actions) = self.redo_stack.pop() {
            let mut undo_actions = Vec::new();
            for action in &actions {
                match action {
                    EditAction::InsertChar(pos, ch) => {
                        self.buffer.insert(*pos, *ch);
                        self.cursor = pos + 1;
                        undo_actions.push(EditAction::DeleteChar(*pos, *ch));
                    }
                    EditAction::InsertStr(pos, text) => {
                        let chars: Vec<char> = text.chars().collect();
                        let len = chars.len();
                        for (i, ch) in chars.into_iter().enumerate() {
                            self.buffer.insert(pos + i, ch);
                        }
                        self.cursor = pos + len;
                        undo_actions.push(EditAction::DeleteStr(*pos, text.clone()));
                    }
                    EditAction::DeleteChar(pos, ch) => {
                        self.buffer.remove(*pos);
                        self.cursor = *pos;
                        undo_actions.push(EditAction::InsertChar(*pos, *ch));
                    }
                    EditAction::DeleteStr(pos, text) => {
                        let len = text.chars().count();
                        self.buffer.drain(*pos..*pos + len);
                        self.cursor = *pos;
                        undo_actions.push(EditAction::InsertStr(*pos, text.clone()));
                    }
                }
            }
            self.undo_stack.push(undo_actions);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.commit_transaction();
        let killed: String = self.buffer.drain(..).collect();
        if !killed.is_empty() {
            self.undo_stack.push(vec![EditAction::DeleteStr(0, killed)]);
            self.redo_stack.clear();
        }
        self.cursor = 0;
    }

    pub fn visual_cursor_col(&self) -> usize {
        self.buffer[..self.cursor]
            .iter()
            .map(|ch| ch.width().unwrap_or(0))
            .sum()
    }

    pub fn visual_width(&self) -> usize {
        self.buffer.iter().map(|ch| ch.width().unwrap_or(0)).sum()
    }
}

impl std::str::FromStr for LineBuffer {
    type Err = std::convert::Infallible;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_text(text))
    }
}
