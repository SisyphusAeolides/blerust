use std::env;
use std::fs;
use std::process::Command;

pub fn get_prompt() -> String {
    // Matches _configure_prompt() in ~/.bashrc exactly
    let c_yellow = "\x1b[1;38;2;215;153;33m"; // Warm Golden Yellow (#d79921)
    let c_blue   = "\x1b[1;34m";              // Adwaita Blue (#78aeed)
    let c_teal   = "\x1b[1;38;2;46;194;126m"; // Adwaita Teal (#2ec27e)
    let c_reset  = "\x1b[0m";
    let c_bold   = "\x1b[1m";

    //  — Fedora glyph (Nerd Fonts U+F30A)
    let fedora = "\u{F30A}";

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

    // Git branch — shown in blue inside parens, matching GIT_INFO=" ($branch)"
    let mut git_info = String::new();
    if let Ok(out) = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        && out.status.success()
    {
        let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !branch.is_empty() {
            git_info = format!(" ({}{}{}{c_blue})", c_blue, branch, c_yellow);
        }
    }

    // Mirrors PS1 from .bashrc:
    // \n{YELLOW}╭─ {BLUE}{fedora} {user}{YELLOW}@{BLUE}{host} {YELLOW}: {TEAL}{cwd} {BLUE}{git_info}
    // \n{YELLOW}╰─λ {RESET}{BOLD}
    format!(
        "\n{c_yellow}╭─ {c_blue}{fedora} {user}{c_yellow}@{c_blue}{host} {c_yellow}: {c_teal}{cwd} {c_blue}{git_info}\n{c_yellow}╰─λ {c_reset}{c_bold}"
    )
}
