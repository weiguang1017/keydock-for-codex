use std::path::PathBuf;
use std::process::Command;

use crate::validate::trim;

pub fn find_codex_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("CKM_CODEX_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    if let Ok(path) = shell_lookup() {
        if path.exists() {
            return Ok(path);
        }
    }

    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
        home.join(".local/bin/codex"),
        home.join(".nvm/current/bin/codex"),
    ];
    let nvm_root = home.join(".nvm/versions/node");
    if let Ok(entries) = std::fs::read_dir(nvm_root) {
        let mut versions = entries
            .flatten()
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        versions.sort();
        versions.reverse();
        for version in versions {
            candidates.push(version.join("bin/codex"));
        }
    }

    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| "Codex CLI was not found. Configure codex in your shell first.".to_string())
}

pub fn read_codex_login(codex_path: &PathBuf) -> Result<String, String> {
    let output = Command::new(codex_path)
        .args(["login", "status"])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(trim(String::from_utf8_lossy(&output.stdout)))
    } else {
        let stderr = trim(String::from_utf8_lossy(&output.stderr));
        Err(if stderr.is_empty() {
            "codex login status failed.".to_string()
        } else {
            stderr
        })
    }
}

pub fn extract_masked_key_from_status(status_output: &str) -> String {
    trim(status_output)
        .split_whitespace()
        .find(|part| part.starts_with("sk-"))
        .unwrap_or("")
        .to_string()
}

/// Returns true when the Codex desktop client appears to be running.
pub fn is_codex_desktop_running() -> bool {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("/usr/bin/osascript")
            .args([
                "-e",
                "tell application \"System Events\" to (count (processes whose bundle identifier is \"com.openai.codex\")) > 0",
            ])
            .output()
        {
            let stdout = trim(String::from_utf8_lossy(&output.stdout));
            if stdout.eq_ignore_ascii_case("true") {
                return true;
            }
            if stdout.eq_ignore_ascii_case("false") {
                return false;
            }
        }
        // Fallback to a plain process-name lookup if the AppleScript probe fails.
        Command::new("/usr/bin/pgrep")
            .args(["-x", "Codex"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("tasklist.exe")
            .args(["/FI", "IMAGENAME eq Codex.exe", "/NH"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains("Codex.exe"))
            .unwrap_or(false)
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        Command::new("pgrep")
            .args(["-x", "Codex"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Restart the Codex desktop client, but only when it is already running.
/// Returns `Ok(true)` when it was running and got restarted, `Ok(false)` when
/// it was not running (and is intentionally left closed rather than launched).
pub fn restart_codex_desktop(_codex_path: Option<&PathBuf>) -> Result<bool, String> {
    if std::env::var("CKM_DISABLE_RESTART").ok().as_deref() == Some("1") {
        return Ok(false);
    }

    if !is_codex_desktop_running() {
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("/usr/bin/osascript")
            .args(["-e", "tell application id \"com.openai.codex\" to quit"])
            .output();
        Command::new("/usr/bin/open")
            .args(["-b", "com.openai.codex"])
            .output()
            .map_err(|error| error.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill.exe")
            .args(["/IM", "Codex.exe", "/F"])
            .output();
        if let Some(codex_path) = _codex_path {
            let _ = Command::new(codex_path).arg("app").output();
        }
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let _ = Command::new("/usr/bin/pkill").args(["-x", "Codex"]).output();
        if let Some(codex_path) = _codex_path {
            let _ = Command::new(codex_path).arg("app").output();
        }
    }

    Ok(true)
}

fn shell_lookup() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd.exe")
        .args(["/c", "where codex"])
        .output()
        .map_err(|error| error.to_string())?;

    #[cfg(not(target_os = "windows"))]
    let output = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        Command::new(shell)
            .args(["-lc", "command -v codex"])
            .output()
            .map_err(|error| error.to_string())?
    };

    if !output.status.success() {
        return Err("codex was not found in PATH.".to_string());
    }
    let stdout = trim(String::from_utf8_lossy(&output.stdout));
    stdout
        .lines()
        .next()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| "codex was not found in PATH.".to_string())
}
