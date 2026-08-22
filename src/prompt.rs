use std::env;
use std::process::Command;
use std::fs;

pub fn get_prompt() -> String {
    let c_rocky = "\x1b[1;38;2;0;168;107m";
    let c_amber = "\x1b[1;38;5;214m";
    let c_ciq_blue = "\x1b[1;38;2;42;115;212m";
    let c_path = "\x1b[1;38;5;75m";
    let c_reset = "\x1b[0m";
    let c_bold = "\x1b[1m";

    let user = env::var("USER").unwrap_or_else(|_| "user".to_string());
    
    let host = fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_else(|_| "host\n".to_string())
        .trim()
        .to_string();

    let mut cwd = env::current_dir().unwrap_or_default().display().to_string();
    if let Ok(home) = env::var("HOME") {
        if cwd.starts_with(&home) {
            cwd = cwd.replacen(&home, "~", 1);
        }
    }

    let mut git_info = String::new();
    if let Ok(out) = Command::new("git").arg("branch").arg("--show-current").output() {
        if out.status.success() {
            let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !branch.is_empty() {
                git_info = format!(" ({}{}{})", c_ciq_blue, branch, c_rocky);
            }
        }
    }

    format!(
        "\r\n{}╭─ {} {}{}@{}{}{} : {} {} {}{}\r\n{}╰─λ {}{}",
        c_amber, c_rocky, user, c_amber, c_rocky, host, c_amber, c_path, cwd, c_rocky, git_info, c_amber, c_reset, c_bold
    )
}
