use std::env;
use std::process::Command;

use blerust::{EditorConfig, LineEditor, ReadlineResult};
mod prompt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = EditorConfig::default();
    config.auto_suggestion = true;
    config.tab_completion = true;
    let mut editor = LineEditor::new(config);

    loop {
        let prompt_str = prompt::get_prompt();

        match editor.readline(&prompt_str)? {
            ReadlineResult::Success(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if trimmed == "exit" || trimmed == "quit" {
                    break;
                }

                if trimmed.starts_with("cd ") || trimmed == "cd" {
                    let target = if trimmed == "cd" {
                        env::var("HOME").unwrap_or_else(|_| "/".to_string())
                    } else {
                        trimmed[3..].trim().to_string()
                    };

                    let path = if target.starts_with('~') {
                        if let Ok(home) = env::var("HOME") {
                            target.replacen('~', &home, 1)
                        } else {
                            target
                        }
                    } else {
                        target
                    };

                    if let Err(e) = env::set_current_dir(&path) {
                        eprintln!("cd: {}: {}", path, e);
                    }
                    continue;
                }

                let status = Command::new("bash")
                    .arg("-c")
                    .arg(trimmed)
                    .status();

                match status {
                    Ok(s) => {
                        if !s.success() {
                            if let Some(code) = s.code() {
                                eprintln!("Process exited with status: {}", code);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Execution error: {}", e);
                    }
                }
            }
            ReadlineResult::Interrupt => {
                continue;
            }
            ReadlineResult::Eof => {
                println!("exit");
                break;
            }
        }
    }

    Ok(())
}
