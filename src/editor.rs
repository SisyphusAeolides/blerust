use std::io::{self, Stdout, Write};

use crossterm::QueueableCommand;
use crossterm::cursor::{self, MoveTo, MoveToColumn};
use crossterm::event::{self, Event};
use crossterm::style::{
    Color, Print, PrintStyledContent, ResetColor, SetForegroundColor, StyledContent,
};
use crossterm::terminal::{self, Clear, ClearType};

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
        let _ = self.stdout.queue(crossterm::event::EnableBracketedPaste);

        let res = self.readline_loop(prompt);

        let _ = self.stdout.queue(crossterm::event::DisableBracketedPaste);
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
                Event::Paste(text) => {
                    let text = normalize_paste(&text);
                    if text.is_empty() {
                        continue;
                    }

                    self.buffer.insert_str(&text);

                    if text.contains('\n') {
                        let command = self.buffer.as_str();
                        self.completion_menu = None;
                        self.render_pasted_block_final(prompt, &command)?;
                        if !command.trim().is_empty() {
                            self.history.add(&command);
                        }
                        return Ok(ReadlineResult::Success(command));
                    } else {
                        // Pasted text is literal input. Completion resumes on
                        // the next typed edit instead of interpreting code or
                        // data from the clipboard as an interactive prefix.
                        self.completion_menu = None;
                        self.render(prompt)?;
                        continue;
                    }
                }
                Event::Key(key_event) => {
                    let action = self.keymap.handle_key(key_event);

                    match action {
                        Action::Submit => {
                            let line = self.buffer.as_str();
                            self.completion_menu = None;
                            self.render_line_final(prompt)?;
                            if !line.trim().is_empty() {
                                self.history.add(&line);
                            }
                            return Ok(ReadlineResult::Success(line));
                        }
                        Action::Interrupt => {
                            self.completion_menu = None;
                            self.stdout.queue(Print("^C\r\n"))?;
                            self.stdout.flush()?;
                            return Ok(ReadlineResult::Interrupt);
                        }
                        Action::Eof => {
                            if self.buffer.is_empty() {
                                self.completion_menu = None;
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
                            if self.config.auto_suggestion
                                && self.buffer.cursor() == self.buffer.len()
                            {
                                let line = self.buffer.as_str();
                                if let Some(suffix) = self.history.suggest_suffix(&line) {
                                    self.buffer.insert_str(&suffix);
                                }
                            } else {
                                self.buffer.move_cursor_right();
                            }
                        }
                        Action::HistoryPrev => {
                            if self.history.search_prefix.is_none() {
                                self.history.search_prefix = Some(self.buffer.as_str().to_string());
                            }
                            let prefix = self.history.search_prefix.as_ref().unwrap().clone();
                            if let Some(matched) = self.history.previous_match(&prefix) {
                                let s = matched.to_string();
                                self.buffer = LineBuffer::from_text(&s);
                            }
                        }
                        Action::HistoryNext => {
                            if self.history.search_prefix.is_none() {
                                self.history.search_prefix = Some(self.buffer.as_str().to_string());
                            }
                            let prefix = self.history.search_prefix.as_ref().unwrap().clone();
                            if let Some(matched) = self.history.next_match(&prefix) {
                                let s = matched.to_string();
                                self.buffer = LineBuffer::from_text(&s);
                            } else {
                                self.buffer = LineBuffer::from_text(&prefix);
                            }
                        }
                        Action::CompleteTab => {
                            if self.config.tab_completion {
                                let line = self.buffer.as_str();
                                let cursor = self.buffer.cursor();
                                let mut handled = false;
                                if let Some((start_idx, candidates)) =
                                    self.completer.complete(&line, cursor)
                                {
                                    if candidates.len() == 1 {
                                        let replacement = &candidates[0];
                                        let current_token: String = line
                                            .chars()
                                            .skip(start_idx)
                                            .take(cursor.saturating_sub(start_idx))
                                            .collect();
                                        if let Some(addition) =
                                            replacement.strip_prefix(&current_token)
                                        {
                                            self.buffer.insert_str(addition);
                                        }
                                        self.completion_menu = None;
                                        handled = true;
                                    } else if candidates.len() > 1 {
                                        let lcp = Completer::longest_common_prefix(&candidates);
                                        let current_token: String = line
                                            .chars()
                                            .skip(start_idx)
                                            .take(cursor.saturating_sub(start_idx))
                                            .collect();
                                        let current_chars = current_token.chars().count();
                                        if lcp.chars().count() > current_chars
                                            && lcp.starts_with(&current_token)
                                        {
                                            let addition: String =
                                                lcp.chars().skip(current_chars).collect();
                                            self.buffer.insert_str(&addition);
                                        }
                                        handled = true;
                                        // Menu is already populated by auto-complete trigger
                                    }
                                }

                                // Fallback: if no completion candidates, accept shadow suggestion
                                if !handled
                                    && self.config.auto_suggestion
                                    && cursor == line.chars().count()
                                    && let Some(suffix) = self.history.suggest_suffix(&line)
                                {
                                    self.buffer.insert_str(&suffix);
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
        if cursor > 0
            && line
                .chars()
                .nth(cursor - 1)
                .is_some_and(|c| c.is_whitespace())
        {
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
        let last_line = prompt.split('\n').next_back().unwrap_or(prompt);
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
                self.stdout
                    .queue(cursor::MoveUp(self.rendered_row_offset))?;
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
                self.stdout.queue(PrintStyledContent(StyledContent::new(
                    span.style, span.text,
                )))?;
                self.stdout.queue(ResetColor)?;
            }
        } else {
            self.stdout.queue(Print(&line))?;
        }

        let mut suffix_len = 0;
        if self.config.auto_suggestion
            && self.buffer.cursor() == self.buffer.len()
            && let Some(suffix) = self.history.suggest_suffix(&line)
        {
            suffix_len = unicode_width::UnicodeWidthStr::width(suffix.as_str());
            self.stdout.queue(SetForegroundColor(Color::DarkGrey))?;
            self.stdout.queue(Print(&suffix))?;
            self.stdout.queue(ResetColor)?;
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
            self.stdout.queue(Print("\r\n"))?;
            self.stdout.queue(Clear(ClearType::CurrentLine))?;

            let display_candidates: Vec<&str> =
                candidates.iter().take(8).map(|s| s.as_str()).collect();
            let mut menu_str = display_candidates.join("   ");

            let prefix = "  [ ";
            let suffix_str = if candidates.len() > 8 {
                format!(" ... +{} more ]", candidates.len() - 8)
            } else {
                " ]".to_string()
            };

            let max_len = (cols as usize).saturating_sub(prefix.len() + suffix_str.len() + 1);
            if menu_str.len() > max_len {
                menu_str.truncate(max_len);
            }

            self.stdout.queue(SetForegroundColor(Color::DarkCyan))?;
            self.stdout.queue(Print(prefix))?;
            self.stdout.queue(SetForegroundColor(Color::Yellow))?;
            self.stdout.queue(Print(&menu_str))?;
            self.stdout.queue(SetForegroundColor(Color::DarkCyan))?;
            self.stdout.queue(Print(&suffix_str))?;
            self.stdout.queue(ResetColor)?;

            self.stdout.queue(cursor::MoveUp(1))?;
        }

        if rows_to_move_up > 0 {
            self.stdout.queue(cursor::MoveUp(rows_to_move_up))?;
        }
        self.stdout.queue(MoveToColumn(target_col_offset))?;
        self.rendered_row_offset = target_row_offset;
        self.stdout.flush()?;
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
            self.stdout
                .queue(cursor::MoveDown(end_row_offset - target_row_offset))?;
        }

        self.stdout.queue(ResetColor)?;
        self.stdout.queue(Print("\r\n"))?;
        self.stdout.queue(Clear(ClearType::CurrentLine))?;
        self.stdout.flush()?;
        Ok(())
    }

    fn render_pasted_block_final(&mut self, prompt: &str, command: &str) -> io::Result<()> {
        self.stdout.queue(Print("\r"))?;
        self.stdout.queue(Clear(ClearType::FromCursorDown))?;
        self.stdout.queue(Print(prompt))?;
        self.stdout.queue(Print(command.replace('\n', "\r\n")))?;
        self.stdout.queue(ResetColor)?;
        if !command.ends_with('\n') {
            self.stdout.queue(Print("\r\n"))?;
        }
        self.stdout.flush()?;
        Ok(())
    }
}

fn normalize_paste(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::normalize_paste;

    #[test]
    fn preserves_multiple_commands_as_one_block() {
        let pasted = "printf 'one\\n'\nprintf 'two\\n'\n";
        assert_eq!(normalize_paste(pasted), pasted);
    }

    #[test]
    fn normalizes_crlf_without_discarding_commands() {
        assert_eq!(
            normalize_paste("echo one\r\necho two\r\n"),
            "echo one\necho two\n"
        );
    }

    #[test]
    fn preserves_heredoc_structure() {
        let pasted = "cat <<'EOF'\nfirst\nsecond\nEOF\n";
        assert_eq!(normalize_paste(pasted), pasted);
    }
}
