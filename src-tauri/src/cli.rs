use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::validate::trim;

pub fn find_codex_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("CKM_CODEX_PATH") {
        let path = PathBuf::from(path);
        if is_executable_file(&path) {
            return Ok(path);
        }
    }

    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    let path_dirs = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    let application_dirs = if cfg!(target_os = "macos") {
        vec![PathBuf::from("/Applications"), home.join("Applications")]
    } else {
        Vec::new()
    };
    // Finder-launched apps do not inherit terminal PATH or shell setup. Look
    // for real installations directly before trying a login shell.
    let mut candidates = codex_install_candidates(&home, &path_dirs, &application_dirs);
    for directory in common_command_dirs() {
        append_codex_commands(&mut candidates, &directory);
    }
    if let Some(path) = candidates.into_iter().find(|path| is_executable_file(path)) {
        return Ok(path);
    }

    shell_lookup().map_err(|_| {
        "Codex executable was not found in PATH, desktop app bundles, or local installations. Install Codex/ChatGPT Desktop or set CKM_CODEX_PATH to the Codex executable.".to_string()
    })
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn codex_command_names() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["codex.exe", "codex.cmd", "codex"]
    } else {
        &["codex"]
    }
}

fn append_codex_commands(candidates: &mut Vec<PathBuf>, directory: &Path) {
    candidates.extend(
        codex_command_names()
            .iter()
            .map(|name| directory.join(name)),
    );
}

fn codex_install_candidates(
    home: &Path,
    path_dirs: &[PathBuf],
    application_dirs: &[PathBuf],
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for directory in path_dirs {
        append_codex_commands(&mut candidates, directory);
    }
    for directory in application_dirs {
        for app in ["Codex.app", "ChatGPT.app"] {
            let resources = directory.join(app).join("Contents/Resources");
            // Standalone Codex Desktop and the Codex runtime inside ChatGPT
            // ship native executables; these do not need a Node.js install.
            candidates.push(resources.join("codex"));
            candidates.push(resources.join("codex-cli/CodexCLI.app/Contents/MacOS/codex"));
            candidates.push(resources.join("codex-cli/bin/codex"));
        }
    }
    append_versioned_codex_commands(
        &mut candidates,
        &home.join(".codex/packages/app-server-daemon/releases"),
    );
    for directory in [
        ".hermes/node/bin",
        ".local/bin",
        ".codex/bin",
        ".cargo/bin",
        ".npm-global/bin",
        ".volta/bin",
        ".nvm/current/bin",
        "Library/pnpm",
    ] {
        append_codex_commands(&mut candidates, &home.join(directory));
    }
    append_versioned_codex_commands(&mut candidates, &home.join(".nvm/versions/node"));
    candidates
}

fn append_versioned_codex_commands(candidates: &mut Vec<PathBuf>, root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut versions = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    let version_numbers = |path: &Path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .trim_start_matches('v')
            .split('-')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    versions.sort_by(|left, right| {
        version_numbers(right)
            .cmp(&version_numbers(left))
            .then_with(|| right.cmp(left))
    });
    for version in versions {
        append_codex_commands(candidates, &version.join("bin"));
    }
}

pub fn read_codex_login(codex_path: &PathBuf) -> Result<String, String> {
    let mut command = Command::new(codex_path);
    apply_command_path(&mut command, &runtime_dirs_for_path(codex_path));
    let output = command
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

pub fn apply_command_path(command: &mut Command, extra_dirs: &[PathBuf]) {
    if let Some(path) = command_path(extra_dirs) {
        command.env("PATH", path);
    }
}

pub fn runtime_dirs_for_path(path: &Path) -> Vec<PathBuf> {
    path.parent()
        .map(|parent| vec![parent.to_path_buf()])
        .unwrap_or_default()
}

fn command_path(extra_dirs: &[PathBuf]) -> Option<OsString> {
    let mut dirs = Vec::new();
    push_dirs(&mut dirs, std::env::var_os("PATH"));
    push_dirs(&mut dirs, login_shell_path());
    for dir in extra_dirs {
        push_dir(&mut dirs, dir.clone());
    }
    for dir in common_command_dirs() {
        push_dir(&mut dirs, dir);
    }
    std::env::join_paths(dirs).ok()
}

fn push_dirs(dirs: &mut Vec<PathBuf>, value: Option<OsString>) {
    if let Some(value) = value {
        for dir in std::env::split_paths(&value) {
            push_dir(dirs, dir);
        }
    }
}

fn push_dir(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if dir.as_os_str().is_empty() {
        return;
    }
    if dirs.iter().any(|item| item == &dir) {
        return;
    }
    dirs.push(dir);
}

fn common_command_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".hermes/node/bin"));
        dirs.push(home.join(".nvm/current/bin"));
        // An npm shim discovered under another Node installation needs its
        // sibling `node`, even when the GUI's inherited PATH is minimal.
        if let Ok(entries) = std::fs::read_dir(home.join(".nvm/versions/node")) {
            for entry in entries.flatten() {
                dirs.push(entry.path().join("bin"));
            }
        }
    }
    dirs
}

fn login_shell_path() -> Option<OsString> {
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd.exe")
        .args(["/c", "echo %PATH%"])
        .output()
        .ok()?;

    #[cfg(not(target_os = "windows"))]
    let output = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        Command::new(shell)
            .args(["-lc", "printf %s \"$PATH\""])
            .output()
            .ok()?
    };

    if !output.status.success() {
        return None;
    }
    let value = trim(String::from_utf8_lossy(&output.stdout));
    if value.is_empty() {
        None
    } else {
        Some(OsString::from(value))
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
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .to_lowercase()
                    .contains("codex.exe")
            })
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
                String::from_utf8_lossy(&output.stdout).lines().any(|line| {
                    let lower = line.to_lowercase();
                    // Skip the desktop app process and our own manager process.
                    if lower.contains("codex.app")
                        || lower.contains("keydock")
                        || lower.contains("codex helper")
                    {
                        return false;
                    }
                    // Match an invocation whose executable is literally `codex`.
                    line.split_whitespace().nth(1).is_some_and(|cmd| {
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
        let _ = Command::new("taskkill.exe")
            .args(["/IM", "Codex.exe"])
            .output();
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
            let _ = Command::new("/usr/bin/pkill")
                .args(["-x", "Codex"])
                .output();
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
    Err(
        "Codex was closed but could not be relaunched automatically. Start it manually."
            .to_string(),
    )
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
        .map(trim)
        .map(PathBuf::from)
        .find(|path| path.is_absolute() && is_executable_file(path))
        .ok_or_else(|| "codex was not found in PATH.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_executable(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "test executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
    }

    #[test]
    fn discovers_desktop_codex_without_path_or_a_login_shell() {
        let root = std::env::temp_dir().join(format!("keydock-discovery-{}", uuid::Uuid::new_v4()));
        let home = root.join("home");
        let applications = root.join("Applications");
        for relative in [
            "ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
            "Codex.app/Contents/Resources/codex",
        ] {
            let executable = applications.join(relative);
            test_executable(&executable);
            let found = codex_install_candidates(&home, &[], std::slice::from_ref(&applications))
                .into_iter()
                .find(|path| is_executable_file(path));
            assert_eq!(found.as_ref(), Some(&executable));
            std::fs::remove_file(executable).unwrap();
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn discovers_hermes_managed_codex_without_shell_setup() {
        let home =
            std::env::temp_dir().join(format!("keydock-hermes-discovery-{}", uuid::Uuid::new_v4()));
        let executable = home.join(".hermes/node/bin").join(codex_command_names()[0]);
        test_executable(&executable);
        let found = codex_install_candidates(&home, &[], &[])
            .into_iter()
            .find(|path| is_executable_file(path));
        std::fs::remove_dir_all(home).unwrap();
        assert_eq!(found, Some(executable));
    }

    #[test]
    fn discovers_the_newest_app_server_runtime() {
        let home = std::env::temp_dir().join(format!(
            "keydock-runtime-discovery-{}",
            uuid::Uuid::new_v4()
        ));
        let root = home.join(".codex/packages/app-server-daemon/releases");
        let mut newest = PathBuf::new();
        for version in [
            "0.99.0-aarch64-apple-darwin",
            "0.171.0-aarch64-apple-darwin",
        ] {
            let executable = root
                .join(version)
                .join("bin")
                .join(codex_command_names()[0]);
            test_executable(&executable);
            newest = executable;
        }
        let found = codex_install_candidates(&home, &[], &[])
            .into_iter()
            .find(|path| is_executable_file(path));
        std::fs::remove_dir_all(home).unwrap();
        assert_eq!(found, Some(newest));
    }

    #[test]
    fn candidates_reject_directories_and_non_executable_files() {
        let root =
            std::env::temp_dir().join(format!("keydock-invalid-codex-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        assert!(!is_executable_file(&root));
        assert!(!is_executable_file(&root.join("missing")));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file = root.join("codex");
            std::fs::write(&file, "not executable").unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
            assert!(!is_executable_file(&file));
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires a real locally installed Codex; run with a minimal GUI PATH"]
    fn installed_codex_is_discovered_and_starts() {
        let path = find_codex_path().unwrap();
        let output = Command::new(&path).arg("--version").output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        println!(
            "Discovered {}: {}",
            path.display(),
            trim(String::from_utf8_lossy(&output.stdout))
        );
    }

    #[test]
    fn wait_for_polls_until_true_or_timeout() {
        assert!(wait_for(|| true, Duration::from_millis(10)));
        let start = Instant::now();
        assert!(!wait_for(|| false, Duration::from_millis(300)));
        assert!(start.elapsed() >= Duration::from_millis(300));
    }
}
