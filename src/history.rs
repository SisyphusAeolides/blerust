use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub struct History {
    entries: Vec<String>,
    cursor: Option<usize>,
    file_path: Option<PathBuf>,
    max_entries: usize,
    pub search_prefix: Option<String>,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    pub fn new() -> Self {
        let mut history = Self {
            entries: Vec::new(),
            cursor: None,
            file_path: None,
            max_entries: 10000,
            search_prefix: None,
        };

        if let Some(home) = env::var_os("HOME") {
            let path = PathBuf::from(home).join(".bash_history");
            let _ = history.load_from_file(&path);
            history.file_path = Some(path);
        }

        history
    }

    pub fn with_file<P: AsRef<Path>>(path: P) -> Self {
        let mut history = Self {
            entries: Vec::new(),
            cursor: None,
            file_path: Some(path.as_ref().to_path_buf()),
            max_entries: 10000,
            search_prefix: None,
        };
        let _ = history.load_from_file(path);
        history
    }

    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        if !path.as_ref().exists() {
            return Ok(());
        }
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.entries.push(trimmed.to_string());
            }
        }
        if self.entries.len() > self.max_entries {
            let drop_count = self.entries.len() - self.max_entries;
            self.entries.drain(0..drop_count);
        }
        Ok(())
    }

    pub fn add(&mut self, entry: &str) {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            return;
        }

        if self.entries.last().map(|s| s.as_str()) == Some(trimmed) {
            return;
        }

        self.entries.push(trimmed.to_string());
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }

        if let Some(ref path) = self.file_path
            && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path)
        {
            let _ = writeln!(file, "{}", trimmed);
        }

        self.reset_cursor();
    }

    pub fn reset_cursor(&mut self) {
        self.cursor = None;
        self.search_prefix = None;
    }

    pub fn suggest_suffix(&self, prefix: &str) -> Option<String> {
        if prefix.is_empty() {
            return None;
        }
        for entry in self.entries.iter().rev() {
            if entry.starts_with(prefix) && entry.len() > prefix.len() {
                return Some(entry[prefix.len()..].to_string());
            }
        }
        None
    }

    pub fn previous_match(&mut self, prefix: &str) -> Option<&str> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }

        let start_idx = match self.cursor {
            Some(idx) => {
                if idx == 0 {
                    0
                } else {
                    idx - 1
                }
            }
            None => len - 1,
        };

        for i in (0..=start_idx).rev() {
            if self.entries[i].starts_with(prefix) {
                self.cursor = Some(i);
                return Some(&self.entries[i]);
            }
        }

        None
    }

    pub fn next_match(&mut self, prefix: &str) -> Option<&str> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }

        let start_idx = self.cursor? + 1;

        for i in start_idx..len {
            if self.entries[i].starts_with(prefix) {
                self.cursor = Some(i);
                return Some(&self.entries[i]);
            }
        }

        self.cursor = None;
        None
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
