use std::env;
use std::fs;
use std::process::Command;

pub fn get_prompt() -> String {
    // All prompt appearance is controlled by environment variables set in ~/.bashrc.
    // Defaults are plain bold ANSI colors — no Nerd Fonts required.
    // Set these in ~/.bashrc to match your personal theme.

    let c_frame  = env::var("BLERUST_FRAME_COLOR")
        .unwrap_or_else(|_| "\x1b[1;33m".to_string());   // bold yellow
    let c_user   = env::var("BLERUST_USER_COLOR")
        .unwrap_or_else(|_| "\x1b[1;34m".to_string());   // bold blue
    let c_path   = env::var("BLERUST_PATH_COLOR")
        .unwrap_or_else(|_| "\x1b[1;36m".to_string());   // bold cyan
    let c_git    = env::var("BLERUST_GIT_COLOR")
        .unwrap_or_else(|_| c_user.clone());
    let c_icon   = env::var("BLERUST_ICON_COLOR")
        .unwrap_or_else(|_| c_user.clone());
    let c_folder = env::var("BLERUST_FOLDER_COLOR")
        .unwrap_or_else(|_| "\x1b[1;31m".to_string());   // bold red
    let c_reset  = "\x1b[0m";
    let c_bold   = "\x1b[1m";

    // Icon glyphs — empty by default so the prompt works without Nerd Fonts.
    // Set in ~/.bashrc to any character or Nerd Font glyph you prefer.
    let os_icon     = env::var("BLERUST_OS_ICON").unwrap_or_default();
    let folder_icon = env::var("BLERUST_FOLDER_ICON").unwrap_or_default();

    let os_prefix = if os_icon.is_empty() {
        String::new()
    } else {
        format!("{c_icon}{os_icon} ")
    };

    let folder_prefix = if folder_icon.is_empty() {
        String::new()
    } else {
        format!("{c_folder}{folder_icon}")
    };

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
            git_info = format!(" {c_git}({branch})");
        }
    }

    format!(
        "\r\n{c_frame}╭─ {os_prefix}{c_user}{user}{c_frame}@{c_user}{host} {c_frame}: {c_path}{cwd} {folder_prefix}{git_info}\r\n{c_frame}╰─λ {c_reset}{c_bold}"
    )
}
