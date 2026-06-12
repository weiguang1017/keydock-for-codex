use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

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

/// Show the OS-native "save file" dialog with an editable default file name.
/// Returns `Ok(Some(path))` with the chosen location, `Ok(None)` when the user
/// cancels, and `Err` when no dialog mechanism is available on this system.
pub fn choose_save_path(default_name: &str) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "macos")]
    {
        // `choose file name` lets the user pick a folder and edit the file name,
        // starting from the Downloads folder.
        let script = format!(
            "POSIX path of (choose file name with prompt \"Export Keydock keys\" default name \"{}\" default location (path to downloads folder))",
            default_name.replace('"', "")
        );
        let output = Command::new("/usr/bin/osascript")
            .args(["-e", &script])
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            let path = trim(String::from_utf8_lossy(&output.stdout));
            if path.is_empty() {
                return Ok(None);
            }
            return Ok(Some(PathBuf::from(path)));
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Exit code 1 with -128 means the user pressed Cancel.
        if stderr.contains("-128") || stderr.to_lowercase().contains("cancel") {
            return Ok(None);
        }
        Err(trim(stderr))
    }

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $d = New-Object System.Windows.Forms.SaveFileDialog; \
             $d.FileName = '{}'; \
             $d.Filter = 'JSON (*.json)|*.json|All files (*.*)|*.*'; \
             if ($d.ShowDialog() -eq 'OK') {{ Write-Output $d.FileName }}",
            default_name.replace('\'', "")
        );
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .output()
            .map_err(|error| error.to_string())?;
        let path = trim(String::from_utf8_lossy(&output.stdout));
        if !output.status.success() {
            return Err(trim(String::from_utf8_lossy(&output.stderr)));
        }
        if path.is_empty() {
            return Ok(None);
        }
        Ok(Some(PathBuf::from(path)))
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let start = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(std::env::temp_dir)
            .join(default_name);
        let output = Command::new("zenity")
            .args([
                "--file-selection",
                "--save",
                "--confirm-overwrite",
                &format!("--filename={}", start.display()),
            ])
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            let path = trim(String::from_utf8_lossy(&output.stdout));
            if path.is_empty() {
                return Ok(None);
            }
            return Ok(Some(PathBuf::from(path)));
        }
        // zenity exits 1 on cancel without stderr noise.
        let stderr = trim(String::from_utf8_lossy(&output.stderr));
        if stderr.is_empty() {
            return Ok(None);
        }
        Err(stderr)
    }
}

/// Returns true when a Codex CLI process appears to be running. This looks for a
/// running `codex` command-line process specifically, as opposed to the desktop
/// app (which `is_codex_desktop_running` covers). The two are reported
/// independently so the UI can tell them apart even when both run at once.
pub fn is_codex_cli_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("tasklist.exe")
            .args(["/FI", "IMAGENAME eq codex.exe", "/NH"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_lowercase().contains("codex.exe"))
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // `pgrep -fl` matches against the full command line. We then filter out the
        // desktop app bundle so a running GUI client is not mistaken for the CLI.
        let pgrep = if cfg!(target_os = "macos") {
            "/usr/bin/pgrep"
        } else {
            "pgrep"
        };
        Command::new(pgrep)
            .args(["-fl", "codex"])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|line| {
                        let lower = line.to_lowercase();
                        // Skip the desktop app process and our own manager process.
                        if lower.contains("codex.app")
                            || lower.contains("keydock")
                            || lower.contains("codex helper")
                        {
                            return false;
                        }
                        // Match an invocation whose executable is literally `codex`.
                        line.split_whitespace().nth(1).map_or(false, |cmd| {
                            let name = cmd.rsplit('/').next().unwrap_or(cmd);
                            name == "codex" || name.starts_with("codex")
                        })
                    })
            })
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

    // Ask the app to quit gracefully. Quitting is asynchronous: the process keeps
    // running for a moment while it tears down.
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("/usr/bin/osascript")
            .args([
                "-e",
                "with timeout of 5 seconds\ntell application id \"com.openai.codex\" to quit\nend timeout",
            ])
            .output();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill.exe").args(["/IM", "Codex.exe"]).output();
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let _ = Command::new("pkill").args(["-x", "Codex"]).output();
    }

    // Wait until the old instance is actually gone before relaunching. Launching
    // while it is still quitting only "activates" the dying instance, so nothing
    // comes back up once the quit completes — the bug this code exists to avoid.
    if !wait_for(|| !is_codex_desktop_running(), Duration::from_secs(10)) {
        // Graceful quit did not finish in time — force-terminate, then wait again.
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("/usr/bin/pkill").args(["-x", "Codex"]).output();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill.exe")
                .args(["/IM", "Codex.exe", "/F"])
                .output();
        }
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            let _ = Command::new("pkill").args(["-9", "-x", "Codex"]).output();
        }
        if !wait_for(|| !is_codex_desktop_running(), Duration::from_secs(5)) {
            return Err(
                "Codex did not quit in time. Restart it manually to apply the new key.".to_string(),
            );
        }
    }

    // Relaunch and confirm the app actually came back; retry once if it did not.
    for _ in 0..2 {
        launch_codex_desktop(_codex_path)?;
        if wait_for(is_codex_desktop_running, Duration::from_secs(6)) {
            return Ok(true);
        }
    }
    Err("Codex was closed but could not be relaunched automatically. Start it manually.".to_string())
}

/// Poll `condition` every 250 ms until it returns true or `timeout` elapses.
fn wait_for(condition: impl Fn() -> bool, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn launch_codex_desktop(_codex_path: Option<&PathBuf>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/open")
            .args(["-b", "com.openai.codex"])
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(trim(String::from_utf8_lossy(&output.stderr)));
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Some(codex_path) = _codex_path {
            // Use spawn, not output: `codex app` can stay attached to the GUI
            // process and waiting on it would block the switch command.
            Command::new(codex_path)
                .arg("app")
                .spawn()
                .map_err(|error| error.to_string())?;
            Ok(())
        } else {
            Err("Codex CLI was not found, so the app could not be relaunched.".to_string())
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_for_polls_until_true_or_timeout() {
        assert!(wait_for(|| true, Duration::from_millis(10)));
        let start = Instant::now();
        assert!(!wait_for(|| false, Duration::from_millis(300)));
        assert!(start.elapsed() >= Duration::from_millis(300));
    }
}
