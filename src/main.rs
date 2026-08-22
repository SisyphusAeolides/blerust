use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use blerust::{EditorConfig, LineEditor, ReadlineResult};
mod prompt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--install" {
        let home = env::var("HOME").expect("HOME directory not found");
        let bashrc_path = PathBuf::from(&home).join(".bashrc");
        let inject_str = "\n# blerust initialization\n[[ $- == *i* ]] && exec blerust\n";
        
        let contents = fs::read_to_string(&bashrc_path).unwrap_or_default();
        if contents.contains("exec blerust") || contents.contains("blerust") && !contents.contains("fastfetch") {
            // Need a smarter check, just seeing if 'exec blerust' or 'blerust' is at the bottom
            if contents.contains("exec blerust") || contents.contains("\nblerust\n") || contents.ends_with("\nblerust") {
                println!("blerust is already configured in ~/.bashrc");
                return Ok(());
            }
        }
        
        let mut file = OpenOptions::new().append(true).create(true).open(&bashrc_path).expect("Failed to open ~/.bashrc");
        file.write_all(inject_str.as_bytes()).expect("Failed to write to ~/.bashrc");
        println!("Successfully installed blerust to ~/.bashrc");
        return Ok(());
    }

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
