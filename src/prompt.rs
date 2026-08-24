use std::env;
use std::fs;
use std::process::Command;

pub fn get_prompt() -> String {
    // Warm Golden Yellow (#d79921) — frame, @, :, ╭─, ╰─λ
    let c_yellow = "\x1b[1;38;2;215;153;33m";
    // Adwaita Blue (#78aeed) — user, host, git branch
    let c_blue = "\x1b[1;34m";
    // Adwaita Teal (#2ec27e) — current directory path
    let c_teal = "\x1b[1;38;2;46;194;126m";
    let c_reset = "\x1b[0m";
    let c_bold = "\x1b[1m";

    let user = env::var("USER").unwrap_or_else(|_| "user".to_string());

    let host = fs::read_to_string("/proc/sys/kernel/hostname")
        .unwrap_or_else(|_| "host\n".to_string())
        .trim()
        .to_string();

    let mut cwd = env::current_dir().unwrap_or_default().display().to_string();
    if let Ok(home) = env::var("HOME")
        && cwd.starts_with(&home)
    {
        cwd = cwd.replacen(&home, "~", 1);
    }

    let mut git_info = String::new();
    if let Ok(out) = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        && out.status.success()
    {
        let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !branch.is_empty() {
            git_info = format!(" ({}{}{})", c_blue, branch, c_yellow);
        }
    }

    format!(
        "\r\n{}╭─  {}{}{}@{}{}{} : {}{} {}{}{}\r\n{}╰─λ {}{}",
        c_yellow,   // ╭─
        c_blue,     //  (Fedora glyph)
        user,
        c_yellow,   // @
        c_blue,
        host,
        c_yellow,   // :
        c_teal,
        cwd,
        c_yellow,   // git accent base color
        git_info,
        c_reset,
        c_yellow,   // ╰─λ
        c_reset,
        c_bold
    )
}
