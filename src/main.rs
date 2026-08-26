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
        let inject_str = r#"
# blerust prompt appearance — edit these to match your theme.
# All values are optional; omitting one uses the built-in default.
# Colors are raw ANSI escape sequences. Icons require a Nerd Font;
# leave them empty ("") for a clean prompt on any terminal.
#
# export BLERUST_FRAME_COLOR=$'\e[1;33m'    # ╭─ frame and ╰─λ color  (default: bold yellow)
# export BLERUST_USER_COLOR=$'\e[1;34m'     # user@host color          (default: bold blue)
# export BLERUST_PATH_COLOR=$'\e[1;36m'     # working directory color  (default: bold cyan)
# export BLERUST_GIT_COLOR=$'\e[1;34m'      # git branch color         (default: BLERUST_USER_COLOR)
# export BLERUST_ICON_COLOR=$'\e[1;34m'     # OS icon color            (default: BLERUST_USER_COLOR)
# export BLERUST_FOLDER_COLOR=$'\e[1;31m'   # folder icon color        (default: bold red)
# export BLERUST_OS_ICON=''                 # OS glyph, e.g. $'\uf303' for Arch
# export BLERUST_FOLDER_ICON=''             # folder glyph, e.g. $'\uf07b'

# blerust initialization (persistent bash wrapper)
if [[ $- == *i* && -z "$BLERUST_ACTIVE" ]]; then
    export BLERUST_ACTIVE=1
    _blerust_loop() {
        PS1=""
        PROMPT_COMMAND=""
        while true; do
            local cmd
            cmd=$(blerust --readline)
            local ret=$?
            if [ $ret -eq 2 ]; then
                exit
            fi
            if [[ -n "$cmd" ]]; then
                history -s "$cmd"
                eval "$cmd"
            fi
        done
    }
    _blerust_loop
    exit
fi
"#;

        let contents = fs::read_to_string(&bashrc_path).unwrap_or_default();
        if contents.contains("BLERUST_ACTIVE") || contents.contains("_blerust_loop") {
            println!("blerust loop is already configured in ~/.bashrc");
            return Ok(());
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

    if args.len() > 1 && args[1] == "--readline" {
        let prompt_str = prompt::get_prompt();
        match editor.readline(&prompt_str)? {
            ReadlineResult::Success(line) => {
                print!("{}", line);
                std::process::exit(0);
            }
            ReadlineResult::Interrupt => {
                std::process::exit(1);
            }
            ReadlineResult::Eof => {
                std::process::exit(2);
            }
        }
    }

    // Fallback: the old standalone shell loop just in case
    loop {
        let prompt_str = prompt::get_prompt();
        match editor.readline(&prompt_str)? {
            ReadlineResult::Success(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                if trimmed == "exit" || trimmed == "quit" { break; }
                if !trimmed.contains(['\n', '\r']) && (trimmed.starts_with("cd ") || trimmed == "cd") {
                    let target = if trimmed == "cd" {
                        env::var("HOME").unwrap_or_else(|_| "/".to_string())
                    } else {
                        trimmed[3..].trim().to_string()
                    };
                    let path = if target.starts_with('~') {
                        if let Ok(home) = env::var("HOME") {
                            target.replacen('~', &home, 1)
                        } else { target }
                    } else { target };
                    if let Err(e) = env::set_current_dir(&path) {
                        eprintln!("cd: {}: {}", path, e);
                    }
                    continue;
                }
                
                let child_command = format!("unset BLERUST_CHILD; {trimmed}");
                let mut command = Command::new("bash");
                command.arg("-c").arg(child_command).env("BLERUST_CHILD", "1");
                if let Ok(home) = env::var("HOME") {
                    let bashrc = PathBuf::from(home).join(".bashrc");
                    if bashrc.is_file() {
                        command.env("BASH_ENV", bashrc);
                    }
                }
                match command.status() {
                    Ok(s) => if !s.success() && let Some(code) = s.code() { eprintln!("Process exited with status: {}", code); }
                    Err(e) => eprintln!("Execution error: {}", e),
                }
            }
            ReadlineResult::Interrupt => continue,
            ReadlineResult::Eof => { println!("exit"); break; }
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
