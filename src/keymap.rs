use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    Emacs,
    ViInsert,
    ViNormal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    InsertChar(char),
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    MoveHome,
    MoveEnd,
    MoveWordLeft,
    MoveWordRight,
    KillToEnd,
    KillToStart,
    KillWordLeft,
    Yank,
    Undo,
    Redo,
    ClearScreen,
    CompleteTab,
    HistoryPrev,
    HistoryNext,
    AcceptSuggestion,
    Submit,
    Interrupt,
    Eof,
    SwitchMode(EditMode),
    Noop,
}

pub struct Keymap {
    pub mode: EditMode,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            mode: EditMode::Emacs,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match self.mode {
            EditMode::Emacs => self.handle_emacs(key),
            EditMode::ViInsert => self.handle_vi_insert(key),
            EditMode::ViNormal => self.handle_vi_normal(key),
        }
    }

    fn handle_emacs(&mut self, key: KeyEvent) -> Action {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Interrupt,
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::Eof,
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => Action::ClearScreen,
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => Action::MoveHome,
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => Action::MoveEnd,
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => Action::MoveLeft,
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => Action::AcceptSuggestion,
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => Action::KillToEnd,
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => Action::KillToStart,
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => Action::KillWordLeft,
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => Action::Yank,
            (KeyCode::Char('z'), KeyModifiers::CONTROL) | (KeyCode::Char('_'), KeyModifiers::CONTROL) => Action::Undo,
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::Redo,

            (KeyCode::Left, KeyModifiers::NONE) => Action::MoveLeft,
            (KeyCode::Right, KeyModifiers::NONE) => Action::MoveRight,
            (KeyCode::Left, KeyModifiers::CONTROL) => Action::MoveWordLeft,
            (KeyCode::Right, KeyModifiers::CONTROL) => Action::MoveWordRight,
            (KeyCode::Home, KeyModifiers::NONE) => Action::MoveHome,
            (KeyCode::End, KeyModifiers::NONE) => Action::MoveEnd,

            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => Action::HistoryPrev,
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => Action::HistoryNext,

            (KeyCode::Tab, KeyModifiers::NONE) => Action::CompleteTab,
            (KeyCode::Backspace, KeyModifiers::NONE) => Action::Backspace,
            (KeyCode::Delete, KeyModifiers::NONE) => Action::Delete,
            (KeyCode::Enter, KeyModifiers::NONE) => Action::Submit,

            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => Action::InsertChar(c),
            _ => Action::Noop,
        }
    }

    fn handle_vi_insert(&mut self, key: KeyEvent) -> Action {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.mode = EditMode::ViNormal;
                Action::MoveLeft
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Interrupt,
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::Eof,
            (KeyCode::Backspace, KeyModifiers::NONE) => Action::Backspace,
            (KeyCode::Enter, KeyModifiers::NONE) => Action::Submit,
            (KeyCode::Tab, KeyModifiers::NONE) => Action::CompleteTab,
            (KeyCode::Right, KeyModifiers::NONE) => Action::AcceptSuggestion,
            (KeyCode::Up, KeyModifiers::NONE) => Action::HistoryPrev,
            (KeyCode::Down, KeyModifiers::NONE) => Action::HistoryNext,
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => Action::InsertChar(c),
            _ => Action::Noop,
        }
    }

    fn handle_vi_normal(&mut self, key: KeyEvent) -> Action {
        match (key.code, key.modifiers) {
            (KeyCode::Char('i'), KeyModifiers::NONE) => {
                self.mode = EditMode::ViInsert;
                Action::Noop
            }
            (KeyCode::Char('a'), KeyModifiers::NONE) => {
                self.mode = EditMode::ViInsert;
                Action::MoveRight
            }
            (KeyCode::Char('A'), KeyModifiers::SHIFT) => {
                self.mode = EditMode::ViInsert;
                Action::MoveEnd
            }
            (KeyCode::Char('I'), KeyModifiers::SHIFT) => {
                self.mode = EditMode::ViInsert;
                Action::MoveHome
            }
            (KeyCode::Char('h'), KeyModifiers::NONE) | (KeyCode::Left, KeyModifiers::NONE) => Action::MoveLeft,
            (KeyCode::Char('l'), KeyModifiers::NONE) | (KeyCode::Right, KeyModifiers::NONE) => Action::MoveRight,
            (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => Action::HistoryPrev,
            (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => Action::HistoryNext,
            (KeyCode::Char('w'), KeyModifiers::NONE) => Action::MoveWordRight,
            (KeyCode::Char('b'), KeyModifiers::NONE) => Action::MoveWordLeft,
            (KeyCode::Char('0'), KeyModifiers::NONE) => Action::MoveHome,
            (KeyCode::Char('$'), KeyModifiers::SHIFT) => Action::MoveEnd,
            (KeyCode::Char('x'), KeyModifiers::NONE) => Action::Delete,
            (KeyCode::Char('u'), KeyModifiers::NONE) => Action::Undo,
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::Redo,
            (KeyCode::Char('D'), KeyModifiers::SHIFT) => Action::KillToEnd,
            (KeyCode::Enter, KeyModifiers::NONE) => Action::Submit,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Interrupt,
            _ => Action::Noop,
        }
    }
}
