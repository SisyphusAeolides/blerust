use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::cmp::Reverse;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

pub struct Completer {
    builtins: Vec<&'static str>,
    matcher: SkimMatcherV2,
    commands: Arc<RwLock<Option<Vec<String>>>>,
    packages: Arc<RwLock<Option<Vec<String>>>>,
    package_load_started: AtomicBool,
}

impl Default for Completer {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer {
    pub fn new() -> Self {
        let builtins = vec![
            "alias", "bg", "bind", "break", "builtin", "cd", "command", "continue", "declare",
            "dirs", "disown", "echo", "enable", "eval", "exec", "exit", "export", "fc", "fg",
            "getopts", "hash", "help", "history", "jobs", "kill", "let", "local", "popd", "pushd",
            "pwd", "read", "readonly", "return", "set", "shift", "shopt", "source", "suspend",
            "test", "times", "trap", "type", "typeset", "ulimit", "umask", "unalias", "unset",
            "wait",
        ];

        let commands = Arc::new(RwLock::new(None));
        let commands_clone = Arc::clone(&commands);
        let _ = thread::Builder::new()
            .name("blerust-command-index".to_string())
            .spawn(move || {
                let discovered = discover_commands();
                if let Ok(mut lock) = commands_clone.write() {
                    *lock = Some(discovered);
                }
            });

        Self {
            builtins,
            matcher: SkimMatcherV2::default(),
            commands,
            packages: Arc::new(RwLock::new(None)),
            package_load_started: AtomicBool::new(false),
        }
    }

    pub fn complete(&self, line: &str, cursor: usize) -> Option<(usize, Vec<String>)> {
        // The editor cursor is a character index. Keep completion indices in
        // that same coordinate system so Unicode input never slices a UTF-8
        // string at an invalid byte boundary.
        let chars_before: Vec<char> = line.chars().take(cursor).collect();
        if chars_before.is_empty() {
            return None;
        }

        let text_before: String = chars_before.iter().collect();
        let current_word_start = chars_before
            .iter()
            .rposition(|ch| ch.is_whitespace() || matches!(ch, '|' | '&' | ';'))
            .map_or(0, |position| position + 1);
        let current_token: String = chars_before[current_word_start..].iter().collect();
        let is_command_position = current_word_start == 0
            || chars_before[..current_word_start]
                .iter()
                .rev()
                .find(|ch| !ch.is_whitespace())
                .is_some_and(|ch| matches!(ch, '|' | '&' | ';'));

        let mut candidates = Vec::new();

        if let Some(var_prefix) = current_token.strip_prefix('$') {
            for (key, _) in env::vars() {
                if key.starts_with(var_prefix) {
                    candidates.push(format!("${key}"));
                }
            }
        } else if is_command_position && !current_token.contains('/') {
            candidates.extend(self.builtins.iter().map(|builtin| (*builtin).to_string()));
            if let Ok(lock) = self.commands.read()
                && let Some(commands) = &*lock
            {
                candidates.extend(commands.iter().cloned());
            }
        } else {
            let is_package_cmd = text_before
                .split_whitespace()
                .any(|word| matches!(word, "dnf" | "rpm" | "yum" | "apt"));

            if is_package_cmd {
                self.start_package_index();
                if let Ok(lock) = self.packages.read()
                    && let Some(packages) = &*lock
                {
                    candidates.extend(packages.iter().cloned());
                }
            } else {
                candidates.extend(self.complete_paths(&current_token));
            }
        }

        let mut matches: Vec<(i64, String)> = candidates
            .into_iter()
            .filter_map(|candidate| {
                if current_token.is_empty() {
                    Some((0, candidate))
                } else if candidate.starts_with(&current_token) {
                    Some((1000, candidate))
                } else {
                    self.matcher
                        .fuzzy_match(&candidate, &current_token)
                        .map(|score| (score, candidate))
                }
            })
            .collect();

        matches.sort_by_key(|(score, _)| Reverse(*score));
        let mut sorted_matches: Vec<String> = matches
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect();
        sorted_matches.dedup();

        if sorted_matches.is_empty() {
            None
        } else {
            Some((current_word_start, sorted_matches))
        }
    }

    fn start_package_index(&self) {
        if self.package_load_started.swap(true, Ordering::AcqRel) {
            return;
        }

        let packages = Arc::clone(&self.packages);
        let _ = thread::Builder::new()
            .name("blerust-package-index".to_string())
            .spawn(move || {
                let output = std::process::Command::new("rpm")
                    .args(["-qa", "--qf", "%{NAME}\n"])
                    .output();
                if let Ok(output) = output
                    && output.status.success()
                {
                    let mut names: Vec<String> = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .map(str::to_owned)
                        .collect();
                    names.sort();
                    names.dedup();
                    if let Ok(mut lock) = packages.write() {
                        *lock = Some(names);
                    }
                }
            });
    }

    fn complete_paths(&self, prefix: &str) -> Vec<String> {
        let expanded = if prefix == "~" || prefix.starts_with("~/") {
            if let Some(home) = env::var_os("HOME") {
                let home = PathBuf::from(home);
                if prefix == "~" {
                    home
                } else {
                    home.join(prefix.strip_prefix("~/").unwrap_or_default())
                }
            } else {
                PathBuf::from(prefix)
            }
        } else {
            PathBuf::from(prefix)
        };

        let (dir_to_read, base_prefix) = if prefix.ends_with('/') {
            (expanded.clone(), prefix.to_string())
        } else {
            let parent = expanded.parent().unwrap_or_else(|| Path::new("."));
            let dir = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            let base = prefix
                .rfind('/')
                .map_or_else(String::new, |last_slash| prefix[..=last_slash].to_string());
            (dir.to_path_buf(), base)
        };

        fs::read_dir(dir_to_read)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let is_dir = entry
                    .file_type()
                    .map(|file_type| file_type.is_dir())
                    .unwrap_or(false);
                Some(if is_dir {
                    format!("{base_prefix}{name}/")
                } else {
                    format!("{base_prefix}{name}")
                })
            })
            .collect()
    }

    pub fn longest_common_prefix(strings: &[String]) -> &str {
        if strings.is_empty() {
            return "";
        }

        let first = &strings[0];
        let mut max_chars = first.chars().count();
        for candidate in &strings[1..] {
            let common_chars = first
                .chars()
                .zip(candidate.chars())
                .take_while(|(left, right)| left == right)
                .count();
            max_chars = max_chars.min(common_chars);
        }

        let byte_end = first
            .char_indices()
            .nth(max_chars)
            .map_or(first.len(), |(byte_index, _)| byte_index);
        &first[..byte_end]
    }
}

fn discover_commands() -> Vec<String> {
    let mut commands = HashSet::new();
    if let Some(path_var) = env::var_os("PATH") {
        for directory in env::split_paths(&path_var) {
            if let Ok(entries) = fs::read_dir(directory) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        commands.insert(name);
                    }
                }
            }
        }
    }

    let mut commands: Vec<String> = commands.into_iter().collect();
    commands.sort();
    commands
}
