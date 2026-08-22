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
        let inject_str = "\n# blerust initialization\nif [[ $- == *i* && ${BLERUST_CHILD:-0} != 1 ]]; then exec blerust; fi\n";

        let contents = fs::read_to_string(&bashrc_path).unwrap_or_default();
        if contents.contains("exec blerust")
            || contents.contains("blerust") && !contents.contains("fastfetch")
        {
            // Need a smarter check, just seeing if 'exec blerust' or 'blerust' is at the bottom
            if contents.contains("exec blerust")
                || contents.contains("\nblerust\n")
                || contents.ends_with("\nblerust")
            {
                println!("blerust is already configured in ~/.bashrc");
                return Ok(());
            }
        }

        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&bashrc_path)
            .expect("Failed to open ~/.bashrc");
        file.write_all(inject_str.as_bytes())
            .expect("Failed to write to ~/.bashrc");
        println!("Successfully installed blerust to ~/.bashrc");
        return Ok(());
    }

    let config = EditorConfig {
        auto_suggestion: env_flag("BLERUST_AUTO_SUGGESTION", true),
        syntax_highlighting: env_flag("BLERUST_SYNTAX_HIGHLIGHTING", true),
        tab_completion: env_flag("BLERUST_TAB_COMPLETION", true),
        ..EditorConfig::default()
    };
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

                // Source the user's normal shell environment for each command
                // without starting another blerust instance.  This preserves
                // aliases such as `ls --color=auto` while keeping the editor
                // process itself responsive.
                let child_command = format!("unset BLERUST_CHILD; {trimmed}");
                let mut command = Command::new("bash");
                command
                    .arg("-c")
                    .arg(child_command)
                    .env("BLERUST_CHILD", "1");
                if let Ok(home) = env::var("HOME") {
                    let bashrc = PathBuf::from(home).join(".bashrc");
                    if bashrc.is_file() {
                        command.env("BASH_ENV", bashrc);
                    }
                }
                let status = command.status();

                match status {
                    Ok(s) => {
                        if !s.success()
                            && let Some(code) = s.code()
                        {
                            eprintln!("Process exited with status: {}", code);
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

fn env_flag(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(value) => !matches!(
            value.to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        ),
        Err(_) => default,
    }
}
