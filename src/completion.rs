use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;

pub struct Completer {
    builtins: Vec<&'static str>,
    matcher: SkimMatcherV2,
    packages: Arc<RwLock<Option<Vec<String>>>>,
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
            "test", "times", "trap", "type", "typeset", "ulimit", "umask", "unalias", "unset", "wait",
        ];
        
        let packages = Arc::new(RwLock::new(None));
        let packages_clone = Arc::clone(&packages);
        
        thread::spawn(move || {
            if let Ok(out) = std::process::Command::new("rpm").args(&["-qa", "--qf", "%{NAME}\n"]).output() {
                if out.status.success() {
                    let mut pkgs: Vec<String> = String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .map(|s| s.to_string())
                        .collect();
                    pkgs.sort();
                    pkgs.dedup();
                    if let Ok(mut lock) = packages_clone.write() {
                        *lock = Some(pkgs);
                    }
                }
            }
        });

        Self { 
            builtins,
            matcher: SkimMatcherV2::default(),
            packages,
        }
    }

    pub fn complete(&self, line: &str, cursor: usize) -> Option<(usize, Vec<String>)> {
        let text_before = &line[..cursor];
        if text_before.is_empty() {
            return None;
        }

        let words: Vec<&str> = text_before.split_whitespace().collect();
        let ends_with_ws = text_before.ends_with(char::is_whitespace);

        let current_word_start = text_before
            .rfind(|c: char| c.is_whitespace() || c == '|' || c == '&' || c == ';')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let current_token = &text_before[current_word_start..];
        let is_first_word = words.is_empty() || (words.len() == 1 && !ends_with_ws) || (words.len() > 0 && text_before[current_word_start..].starts_with(words[0]));

        let mut candidates = Vec::new();

        if current_token.starts_with('$') {
            let var_prefix = &current_token[1..];
            for (key, _) in env::vars() {
                if key.starts_with(var_prefix) {
                    candidates.push(format!("${}", key));
                }
            }
        } else if is_first_word && !current_token.contains('/') {
            for &builtin in &self.builtins {
                candidates.push(builtin.to_string());
            }

            if let Some(path_var) = env::var_os("PATH") {
                let mut seen = HashSet::new();
                for dir in env::split_paths(&path_var) {
                    if let Ok(entries) = fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            if let Ok(name) = entry.file_name().into_string() {
                                if seen.insert(name.clone()) {
                                    candidates.push(name);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Context aware completions
            let mut is_package_cmd = false;
            if text_before.contains("dnf ") || text_before.contains("rpm ") || text_before.contains("yum ") || text_before.contains("apt ") {
                is_package_cmd = true;
            }

            if is_package_cmd {
                if let Ok(lock) = self.packages.read() {
                    if let Some(ref pkgs) = *lock {
                        candidates.extend(pkgs.clone());
                    }
                }
            } else {
                let path_matches = self.complete_paths(current_token);
                candidates.extend(path_matches);
            }
        }

        // Apply fuzzy matching
        let mut matches: Vec<(i64, String)> = candidates
            .into_iter()
            .filter_map(|c| {
                if current_token.is_empty() {
                    Some((0, c))
                } else if c.starts_with(current_token) {
                    // Exact prefix match gets priority boost
                    Some((1000, c))
                } else if let Some(score) = self.matcher.fuzzy_match(&c, current_token) {
                    Some((score, c))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by score descending
        let mut sorted_matches: Vec<String> = matches.into_iter().map(|(_, c)| c).collect();
        sorted_matches.dedup();

        if sorted_matches.is_empty() {
            None
        } else {
            Some((current_word_start, sorted_matches))
        }
    }

    fn complete_paths(&self, prefix: &str) -> Vec<String> {
        let mut results = Vec::new();
        let expanded = if prefix.starts_with('~') {
            if let Some(home) = env::var_os("HOME") {
                let path_buf = PathBuf::from(home);
                if prefix == "~" {
                    path_buf
                } else {
                    path_buf.join(&prefix[2..])
                }
            } else {
                PathBuf::from(prefix)
            }
        } else {
            PathBuf::from(prefix)
        };

        let (dir_to_read, _filename_prefix, base_prefix) = if prefix.ends_with('/') {
            (expanded.clone(), String::new(), prefix.to_string())
        } else {
            let parent = expanded.parent().unwrap_or_else(|| Path::new("."));
            let dir = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            let fname = expanded
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let base = if let Some(last_slash) = prefix.rfind('/') {
                prefix[..=last_slash].to_string()
            } else {
                String::new()
            };

            (dir.to_path_buf(), fname, base)
        };

        if let Ok(entries) = fs::read_dir(dir_to_read) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    let candidate = if is_dir {
                        format!("{}{}/", base_prefix, name)
                    } else {
                        format!("{}{}", base_prefix, name)
                    };
                    results.push(candidate);
                }
            }
        }

        results
    }

    pub fn longest_common_prefix<'a>(strings: &'a [String]) -> &'a str {
        if strings.is_empty() {
            return "";
        }
        let first = &strings[0];
        let mut max_len = first.len();

        for s in &strings[1..] {
            let common_bytes = first
                .bytes()
                .zip(s.bytes())
                .take_while(|(b1, b2)| b1 == b2)
                .count();
            max_len = max_len.min(common_bytes);
        }

        &first[..max_len]
    }
}
