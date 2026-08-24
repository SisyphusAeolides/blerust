use std::env;
use std::fs;
use std::process::Command;

fn get_os_glyph() -> &'static str {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        let content = content.to_lowercase();
        
        let has_id = |target: &str| -> bool {
            content.lines().any(|line| {
                if line.starts_with("id=") || line.starts_with("id_like=") {
                    let val = line.split('=').nth(1).unwrap_or("").trim_matches(|c| c == '"' || c == '\'');
                    val.split(' ').any(|v| v == target)
                } else {
                    false
                }
            })
        };

        if has_id("fedora") { return "\u{F30A}"; } // Fedora
        if has_id("arch") { return "\u{F303}"; } // Arch Linux
        if has_id("ubuntu") { return "\u{F31B}"; } // Ubuntu
        if has_id("debian") { return "\u{F306}"; } // Debian
        if has_id("rhel") || has_id("centos") || has_id("rocky") || has_id("almalinux") { return "\u{F316}"; } // RHEL/CentOS
        if has_id("suse") || has_id("opensuse") || has_id("opensuse-tumbleweed") { return "\u{F314}"; } // SUSE
        if has_id("alpine") { return "\u{F300}"; } // Alpine
        if has_id("linuxmint") { return "\u{F30E}"; } // Mint
        if has_id("pop") { return "\u{F31E}"; } // Pop!_OS
        if has_id("nixos") { return "\u{F313}"; } // NixOS
        if has_id("gentoo") { return "\u{F30D}"; } // Gentoo
        if has_id("raspbian") { return "\u{F315}"; } // Raspberry Pi
    }
    "\u{F31A}" // Generic Tux Linux penguin fallback
}

pub fn get_prompt() -> String {
    // Colors matching _configure_prompt() in ~/.bashrc
    let c_yellow = "\x1b[1;38;2;215;153;33m"; // Warm Golden Yellow (#d79921)
    let c_blue   = "\x1b[1;34m";              // Adwaita Blue
    let c_red    = "\x1b[1;38;2;237;51;59m";  // Adwaita Red (#ed333b)
    let c_teal   = "\x1b[1;38;2;46;194;126m"; // Adwaita Teal (#2ec27e)
    let c_reset  = "\x1b[0m";
    let c_bold   = "\x1b[1m";

    // OS Glyph (Auto-detected via /etc/os-release) — left of username
    let os_glyph = get_os_glyph();
    // Folder glyph (Nerd Fonts U+F07B) — right of path, in red
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
    // \r\n{YELLOW}╭─ {BLUE}{os_glyph} {user}{YELLOW}@{BLUE}{host} {YELLOW}: {TEAL}{cwd} {RED}{folder}{BLUE}{git_info}
    // \r\n{YELLOW}╰─λ {RESET}{BOLD}
    format!(
        "\r\n{c_yellow}╭─ {c_blue}{os_glyph} {user}{c_yellow}@{c_blue}{host} {c_yellow}: {c_teal}{cwd} {c_red}{folder}{c_blue}{git_info}\r\n{c_yellow}╰─λ {c_reset}{c_bold}"
    )
}
