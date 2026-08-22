use crossterm::style::{Color, ContentStyle};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

static SHELL_BUILTINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "alias", "bg", "bind", "break", "builtin", "cd", "command", "continue", "declare",
        "dirs", "disown", "echo", "enable", "eval", "exec", "exit", "export", "fc", "fg",
        "getopts", "hash", "help", "history", "jobs", "kill", "let", "local", "popd", "pushd",
        "pwd", "read", "readonly", "return", "set", "shift", "shopt", "source", "suspend",
        "test", "times", "trap", "type", "typeset", "ulimit", "umask", "unalias", "unset", "wait",
    ]
    .into_iter()
    .collect()
});

static CMD_CACHE: LazyLock<Mutex<HashMap<String, bool>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn is_command_in_path(cmd: &str) -> bool {
    if SHELL_BUILTINS.contains(cmd) {
        return true;
    }

    if cmd.contains('/') {
        let path = Path::new(cmd);
        return path.is_file() && is_executable(path);
    }

    {
        let cache = CMD_CACHE.lock().unwrap();
        if let Some(&exists) = cache.get(cmd) {
            return exists;
        }
    }

    let exists = if let Some(path_var) = env::var_os("PATH") {
        env::split_paths(&path_var).any(|dir| {
            let full_path = dir.join(cmd);
            full_path.is_file() && is_executable(&full_path)
        })
    } else {
        false
    };

    let mut cache = CMD_CACHE.lock().unwrap();
    cache.insert(cmd.to_string(), exists);
    exists
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = path.metadata() {
        meta.permissions().mode() & 0o111 != 0
    } else {
        false
    }
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    CommandValid,
    CommandInvalid,
    Builtin,
    Flag,
    StringLiteral,
    Variable,
    Operator,
    Comment,
    Directory,
    FilePath,
    Argument,
    DefaultText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub text: String,
    pub token_type: TokenType,
    pub style: ContentStyle,
}

pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self
    }

    pub fn highlight(&self, line: &str) -> Vec<StyledSpan> {
        let mut spans = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut expecting_command = true;

        while i < len {
            let ch = chars[i];

            if ch.is_whitespace() {
                let start = i;
                while i < len && chars[i].is_whitespace() {
                    i += 1;
                }
                let ws_text: String = chars[start..i].iter().collect();
                spans.push(StyledSpan {
                    text: ws_text,
                    token_type: TokenType::DefaultText,
                    style: ContentStyle::new(),
                });
                continue;
            }

            if ch == '#' {
                let comment_text: String = chars[i..].iter().collect();
                let mut style = ContentStyle::new();
                style.foreground_color = Some(Color::DarkGrey);
                spans.push(StyledSpan {
                    text: comment_text,
                    token_type: TokenType::Comment,
                    style,
                });
                break;
            }

            if ch == '|' || ch == '&' || ch == ';' || ch == '<' || ch == '>' {
                let start = i;
                if (ch == '|' || ch == '&' || ch == '>') && i + 1 < len && chars[i + 1] == ch {
                    i += 2;
                } else {
                    i += 1;
                }
                let op_text: String = chars[start..i].iter().collect();
                let mut style = ContentStyle::new();
                style.foreground_color = Some(Color::Magenta);
                spans.push(StyledSpan {
                    text: op_text,
                    token_type: TokenType::Operator,
                    style,
                });
                expecting_command = true;
                continue;
            }

            if ch == '"' || ch == '\'' {
                let quote = ch;
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if chars[i] == '\\' && quote == '"' {
                        escaped = true;
                    } else if chars[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let str_text: String = chars[start..i].iter().collect();
                let mut style = ContentStyle::new();
                style.foreground_color = Some(Color::Yellow);
                spans.push(StyledSpan {
                    text: str_text,
                    token_type: TokenType::StringLiteral,
                    style,
                });
                expecting_command = false;
                continue;
            }

            if ch == '$' {
                let start = i;
                i += 1;
                if i < len && chars[i] == '{' {
                    while i < len && chars[i] != '}' {
                        i += 1;
                    }
                    if i < len && chars[i] == '}' {
                        i += 1;
                    }
                } else {
                    while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                }
                let var_text: String = chars[start..i].iter().collect();
                let mut style = ContentStyle::new();
                style.foreground_color = Some(Color::Rgb { r: 255, g: 165, b: 0 }); // Amber
                spans.push(StyledSpan {
                    text: var_text,
                    token_type: TokenType::Variable,
                    style,
                });
                expecting_command = false;
                continue;
            }

            if ch == '-' {
                let start = i;
                while i < len && !chars[i].is_whitespace() && chars[i] != '|' && chars[i] != '&' && chars[i] != ';' {
                    i += 1;
                }
                let flag_text: String = chars[start..i].iter().collect();
                let mut style = ContentStyle::new();
                style.foreground_color = Some(Color::DarkCyan);
                spans.push(StyledSpan {
                    text: flag_text,
                    token_type: TokenType::Flag,
                    style,
                });
                continue;
            }

            let start = i;
            while i < len
                && !chars[i].is_whitespace()
                && chars[i] != '|'
                && chars[i] != '&'
                && chars[i] != ';'
                && chars[i] != '<'
                && chars[i] != '>'
                && chars[i] != '#'
                && chars[i] != '"'
                && chars[i] != '\''
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();

            if expecting_command {
                expecting_command = false;
                if SHELL_BUILTINS.contains(word.as_str()) {
                    let mut style = ContentStyle::new();
                    style.foreground_color = Some(Color::Rgb { r: 0, g: 200, b: 150 }); // Teal
                    spans.push(StyledSpan {
                        text: word,
                        token_type: TokenType::Builtin,
                        style,
                    });
                } else if is_command_in_path(&word) {
                    let mut style = ContentStyle::new();
                    style.foreground_color = Some(Color::Green);
                    spans.push(StyledSpan {
                        text: word,
                        token_type: TokenType::CommandValid,
                        style,
                    });
                } else {
                    let mut style = ContentStyle::new();
                    style.foreground_color = Some(Color::Red);
                    spans.push(StyledSpan {
                        text: word,
                        token_type: TokenType::CommandInvalid,
                        style,
                    });
                }
            } else {
                let path = Path::new(&word);
                if path.is_dir() {
                    let mut style = ContentStyle::new();
                    style.foreground_color = Some(Color::Blue);
                    spans.push(StyledSpan {
                        text: word,
                        token_type: TokenType::Directory,
                        style,
                    });
                } else if path.is_file() {
                    let mut style = ContentStyle::new();
                    style.foreground_color = Some(Color::Cyan);
                    spans.push(StyledSpan {
                        text: word,
                        token_type: TokenType::FilePath,
                        style,
                    });
                } else {
                    spans.push(StyledSpan {
                        text: word,
                        token_type: TokenType::Argument,
                        style: ContentStyle::new(),
                    });
                }
            }
        }

        spans
    }
}
