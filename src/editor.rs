use std::io::{self, Stdout, Write};

use crossterm::cursor::{self, MoveTo, MoveToColumn, RestorePosition, SavePosition};
use crossterm::event::{self, Event};
use crossterm::style::{Color, Print, PrintStyledContent, ResetColor, SetForegroundColor, StyledContent};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::QueueableCommand;

use crate::buffer::LineBuffer;
use crate::completion::Completer;
use crate::highlight::SyntaxHighlighter;
use crate::history::History;
use crate::keymap::{Action, EditMode, Keymap};

pub struct EditorConfig {
    pub auto_suggestion: bool,
    pub syntax_highlighting: bool,
    pub tab_completion: bool,
    pub edit_mode: EditMode,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            auto_suggestion: true,
            syntax_highlighting: true,
            tab_completion: true,
            edit_mode: EditMode::Emacs,
        }
    }
}

pub struct LineEditor {
    buffer: LineBuffer,
    history: History,
    highlighter: SyntaxHighlighter,
    completer: Completer,
    keymap: Keymap,
    config: EditorConfig,
    stdout: Stdout,
    completion_menu: Option<Vec<String>>,
    rendered_row_offset: u16,
    prompt_rendered: bool,
}

pub enum ReadlineResult {
    Success(String),
    Eof,
    Interrupt,
}

impl Default for LineEditor {
    fn default() -> Self {
        Self::new(EditorConfig::default())
    }
}

impl LineEditor {
    pub fn new(config: EditorConfig) -> Self {
        let mode = config.edit_mode;
        Self {
            buffer: LineBuffer::new(),
            history: History::new(),
            highlighter: SyntaxHighlighter::new(),
            completer: Completer::new(),
            keymap: Keymap { mode },
            config,
            stdout: io::stdout(),
            completion_menu: None,
            rendered_row_offset: 0,
            prompt_rendered: false,
        }
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    pub fn readline(&mut self, prompt: &str) -> io::Result<ReadlineResult> {
        self.buffer.clear();
        self.history.reset_cursor();
        self.completion_menu = None;
        self.prompt_rendered = false;
        self.rendered_row_offset = 0;

        terminal::enable_raw_mode()?;

        let res = self.readline_loop(prompt);

        let _ = terminal::disable_raw_mode();
        let _ = self.stdout.queue(ResetColor);
        let _ = self.stdout.flush();

        res
    }

    fn readline_loop(&mut self, prompt: &str) -> io::Result<ReadlineResult> {
        self.render(prompt)?;

        loop {
            let evt = event::read()?;
            match evt {
                Event::Resize(_cols, _rows) => {
                    self.render(prompt)?;
                    continue;
                }
                Event::Key(key_event) => {
                    let action = self.keymap.handle_key(key_event);

                    match action {
                        Action::Submit => {
                            let line = self.buffer.as_str();
                            self.clear_completion_menu()?;
                            self.render_line_final(prompt)?;
                            if !line.trim().is_empty() {
                                self.history.add(&line);
                            }
                            return Ok(ReadlineResult::Success(line));
                        }
                        Action::Interrupt => {
                            self.clear_completion_menu()?;
                            self.stdout.queue(Print("^C\r\n"))?;
                            self.stdout.flush()?;
                            return Ok(ReadlineResult::Interrupt);
                        }
                        Action::Eof => {
                            if self.buffer.is_empty() {
                                self.clear_completion_menu()?;
                                self.stdout.queue(Print("\r\n"))?;
                                self.stdout.flush()?;
                                return Ok(ReadlineResult::Eof);
                            }
                        }
                        Action::InsertChar(ch) => {
                            self.buffer.insert_char(ch);
                            if self.config.tab_completion {
                                self.trigger_autocomplete();
                            } else {
                                self.completion_menu = None;
                            }
                        }
                        Action::Backspace => {
                            self.buffer.backspace();
                            if self.config.tab_completion {
                                self.trigger_autocomplete();
                            } else {
                                self.completion_menu = None;
                            }
                        }
                        Action::Delete => {
                            self.completion_menu = None;
                            self.buffer.delete();
                        }
                        Action::MoveLeft => {
                            self.completion_menu = None;
                            self.buffer.move_cursor_left();
                        }
                        Action::MoveRight => {
                            self.completion_menu = None;
                            self.buffer.move_cursor_right();
                        }
                        Action::MoveHome => {
                            self.completion_menu = None;
                            self.buffer.move_cursor_home();
                        }
                        Action::MoveEnd => {
                            self.completion_menu = None;
                            self.buffer.move_cursor_end();
                        }
                        Action::MoveWordLeft => {
                            self.completion_menu = None;
                            self.buffer.move_word_left();
                        }
                        Action::MoveWordRight => {
                            self.completion_menu = None;
                            self.buffer.move_word_right();
                        }
                        Action::KillToEnd => {
                            self.completion_menu = None;
                            self.buffer.kill_to_end();
                        }
                        Action::KillToStart => {
                            self.completion_menu = None;
                            self.buffer.kill_to_start();
                        }
                        Action::KillWordLeft => {
                            self.completion_menu = None;
                            self.buffer.kill_word_left();
                        }
                        Action::Yank => {
                            self.completion_menu = None;
                            self.buffer.yank();
                        }
                        Action::Undo => {
                            self.completion_menu = None;
                            self.buffer.undo();
                        }
                        Action::Redo => {
                            self.completion_menu = None;
                            self.buffer.redo();
                        }
                        Action::ClearScreen => {
                            self.stdout.queue(Clear(ClearType::All))?;
                            self.stdout.queue(MoveTo(0, 0))?;
                        }
                        Action::AcceptSuggestion => {
                            if self.config.auto_suggestion && self.buffer.cursor() == self.buffer.len() {
                                let line = self.buffer.as_str();
                                if let Some(suffix) = self.history.suggest_suffix(&line) {
                                    self.buffer.insert_str(&suffix);
                                }
                            } else {
                                self.buffer.move_cursor_right();
                            }
                        }
                        Action::HistoryPrev => {
                            let prefix = self.buffer.as_str();
                            if let Some(matched) = self.history.previous_match(&prefix) {
                                let s = matched.to_string();
                                self.buffer = LineBuffer::from_str(&s);
                            }
                        }
                        Action::HistoryNext => {
                            let prefix = self.buffer.as_str();
                            if let Some(matched) = self.history.next_match(&prefix) {
                                let s = matched.to_string();
                                self.buffer = LineBuffer::from_str(&s);
                            } else {
                                self.buffer.clear();
                            }
                        }
                        Action::CompleteTab => {
                            if self.config.tab_completion {
                                let line = self.buffer.as_str();
                                let cursor = self.buffer.cursor();
                                if let Some((start_idx, candidates)) = self.completer.complete(&line, cursor) {
                                    if candidates.len() == 1 {
                                        let replacement = &candidates[0];
                                        let current_token = &line[start_idx..cursor];
                                        if replacement.starts_with(current_token) {
                                            let addition = &replacement[current_token.len()..];
                                            self.buffer.insert_str(addition);
                                        }
                                        self.completion_menu = None;
                                    } else if candidates.len() > 1 {
                                        let lcp = Completer::longest_common_prefix(&candidates);
                                        let current_token = &line[start_idx..cursor];
                                        if lcp.len() > current_token.len() && lcp.starts_with(current_token) {
                                            let addition = &lcp[current_token.len()..];
                                            self.buffer.insert_str(addition);
                                        }
                                        // Menu is already populated by auto-complete trigger
                                    }
                                }
                            }
                        }
                        Action::SwitchMode(new_mode) => {
                            self.keymap.mode = new_mode;
                        }
                        Action::Noop => {}
                    }
                    self.render(prompt)?;
                }
                _ => {}
            }
        }
    }

    fn trigger_autocomplete(&mut self) {
        let line = self.buffer.as_str();
        let cursor = self.buffer.cursor();
        
        // Don't auto-trigger on space, it dumps the entire directory
        if cursor > 0 && line[..cursor].chars().last().map_or(false, |c| c.is_whitespace()) {
            self.completion_menu = None;
            return;
        }
        
        if let Some((_, candidates)) = self.completer.complete(&line, cursor) {
            if candidates.len() > 1 {
                self.completion_menu = Some(candidates);
            } else {
                self.completion_menu = None;
            }
        } else {
            self.completion_menu = None;
        }
    }

    fn prompt_visual_width(prompt: &str) -> usize {
        let last_line = prompt.split('\n').last().unwrap_or(prompt);
        let mut width = 0;
        let mut in_ansi = false;
        for ch in last_line.chars() {
            if ch == '\x1b' {
                in_ansi = true;
            } else if in_ansi {
                if ch.is_ascii_alphabetic() {
                    in_ansi = false;
                }
            } else {
                width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            }
        }
        width
    }

    fn render(&mut self, prompt: &str) -> io::Result<()> {
        let line = self.buffer.as_str();
        let prompt_width = Self::prompt_visual_width(prompt);
        
        if self.prompt_rendered {
            if self.rendered_row_offset > 0 {
                self.stdout.queue(cursor::MoveUp(self.rendered_row_offset))?;
            }
            self.stdout.queue(Print("\r"))?;
            self.stdout.queue(MoveToColumn(prompt_width as u16))?;
            self.stdout.queue(Clear(ClearType::FromCursorDown))?;
        } else {
            self.stdout.queue(Print("\r"))?;
            self.stdout.queue(Clear(ClearType::FromCursorDown))?;
            self.stdout.queue(Print(prompt))?;
            self.prompt_rendered = true;
        }
        
        if self.config.syntax_highlighting {
            let spans = self.highlighter.highlight(&line);
            for span in spans {
                self.stdout.queue(PrintStyledContent(StyledContent::new(span.style, span.text)))?;
            }
        } else {
            self.stdout.queue(Print(&line))?;
        }
        
        let mut suffix_len = 0;
        if self.config.auto_suggestion && self.buffer.cursor() == self.buffer.len() {
            if let Some(suffix) = self.history.suggest_suffix(&line) {
                suffix_len = unicode_width::UnicodeWidthStr::width(suffix.as_str());
                self.stdout.queue(SetForegroundColor(Color::DarkGrey))?;
                self.stdout.queue(Print(&suffix))?;
                self.stdout.queue(ResetColor)?;
            }
        }
        
        let (cols, _rows) = terminal::size().unwrap_or((80, 24));
        let cols = cols.max(1);
        
        let prompt_width = Self::prompt_visual_width(prompt);
        let cursor_visual_offset = self.buffer.visual_cursor_col();
        let line_visual_offset = self.buffer.visual_width();
        let total_offset = prompt_width + cursor_visual_offset;
        let end_offset = prompt_width + line_visual_offset + suffix_len;
        
        let target_row_offset = (total_offset / cols as usize) as u16;
        let target_col_offset = (total_offset % cols as usize) as u16;
        let end_row_offset = (end_offset / cols as usize) as u16;
        let rows_to_move_up = end_row_offset - target_row_offset;

        if let Some(ref candidates) = self.completion_menu {
            self.stdout.queue(SavePosition)?;
            self.stdout.queue(Print("\r\n"))?;
            self.stdout.queue(Clear(ClearType::CurrentLine))?;

            let display_candidates: Vec<&str> = candidates.iter().take(8).map(|s| s.as_str()).collect();
            let menu_str = display_candidates.join("   ");
            self.stdout.queue(SetForegroundColor(Color::DarkCyan))?;
            self.stdout.queue(Print("  [ "))?;
            self.stdout.queue(SetForegroundColor(Color::Yellow))?;
            self.stdout.queue(Print(menu_str))?;
            if candidates.len() > 8 {
                self.stdout.queue(SetForegroundColor(Color::DarkGrey))?;
                self.stdout.queue(Print(format!(" ... +{} more", candidates.len() - 8)))?;
            }
            self.stdout.queue(SetForegroundColor(Color::DarkCyan))?;
            self.stdout.queue(Print(" ]"))?;
            self.stdout.queue(ResetColor)?;
            self.stdout.queue(RestorePosition)?;
        }
        
        if rows_to_move_up > 0 {
            self.stdout.queue(cursor::MoveUp(rows_to_move_up))?;
        }
        self.stdout.queue(MoveToColumn(target_col_offset))?;
        self.rendered_row_offset = target_row_offset;
        self.stdout.flush()?;
        Ok(())
    }

    fn clear_completion_menu(&mut self) -> io::Result<()> {
        if self.completion_menu.is_some() {
            self.stdout.queue(SavePosition)?;
            self.stdout.queue(Print("\r\n"))?;
            self.stdout.queue(Clear(ClearType::CurrentLine))?;
            self.stdout.queue(RestorePosition)?;
            self.completion_menu = None;
        }
        Ok(())
    }

    fn render_line_final(&mut self, _prompt: &str) -> io::Result<()> {
        let (cols, _) = terminal::size().unwrap_or((80, 24));
        let cols = cols.max(1);
        
        let prompt_width = Self::prompt_visual_width(_prompt);
        let cursor_visual_offset = self.buffer.visual_cursor_col();
        let line_visual_offset = self.buffer.visual_width();
        let total_offset = prompt_width + cursor_visual_offset;
        let end_offset = prompt_width + line_visual_offset;
        
        let target_row_offset = (total_offset / cols as usize) as u16;
        let end_row_offset = (end_offset / cols as usize) as u16;
        
        if end_row_offset > target_row_offset {
            self.stdout.queue(cursor::MoveDown(end_row_offset - target_row_offset))?;
        }
        
        self.stdout.queue(ResetColor)?;
        self.stdout.queue(Print("\r\n"))?;
        self.stdout.flush()?;
        Ok(())
    }
}
