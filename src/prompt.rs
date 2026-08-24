use std::env;
use std::fs;
use std::process::Command;

pub fn get_prompt() -> String {
    // Colors matching _configure_prompt() in ~/.bashrc
    let c_yellow = "\x1b[1;38;2;215;153;33m"; // Warm Golden Yellow (#d79921)
    let c_blue   = "\x1b[1;34m";              // Adwaita Blue
    let c_red    = "\x1b[1;38;2;237;51;59m";  // Adwaita Red (#ed333b)
    let c_teal   = "\x1b[1;38;2;46;194;126m"; // Adwaita Teal (#2ec27e)
    let c_reset  = "\x1b[0m";
    let c_bold   = "\x1b[1m";

    //  Fedora glyph (Nerd Fonts U+F30A) — left of username
    let fedora = "\u{F30A}";
    //  Folder glyph (Nerd Fonts U+F07B) — right of path, in red
    let folder = "\u{F07B}";

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

    // Git branch in blue inside parens, matching GIT_INFO=" ($branch)"
    let mut git_info = String::new();
    if let Ok(out) = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        && out.status.success()
    {
        let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !branch.is_empty() {
            git_info = format!(" ({branch})");
        }
    }

    // Matches the ble.sh PS1 exactly:
    // \r\n{YELLOW}╭─ {BLUE}{fedora} {user}{YELLOW}@{BLUE}{host} {YELLOW}: {TEAL}{cwd} {RED}{folder}{BLUE}{git_info}
    // \r\n{YELLOW}╰─λ {RESET}{BOLD}
    format!(
        "\r\n{c_yellow}╭─ {c_blue}{fedora} {user}{c_yellow}@{c_blue}{host} {c_yellow}: {c_teal}{cwd} {c_red}{folder}{c_blue}{git_info}\r\n{c_yellow}╰─λ {c_reset}{c_bold}"
    )
}
