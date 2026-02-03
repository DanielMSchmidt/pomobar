//! Launch agent management for macOS "Start at Login" functionality.
//!
//! Creates and removes a LaunchAgent plist file in ~/Library/LaunchAgents/
//! to enable automatic startup at login.

use directories::BaseDirs;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

const PLIST_NAME: &str = "com.pomobar.plist";

#[derive(Error, Debug)]
pub enum LaunchAgentError {
    #[error("Could not determine home directory")]
    NoHomeDir,
    #[error("Could not determine executable path: {0}")]
    NoExePath(io::Error),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Returns the path to the LaunchAgents directory.
fn launch_agents_dir() -> Result<PathBuf, LaunchAgentError> {
    let base = BaseDirs::new().ok_or(LaunchAgentError::NoHomeDir)?;
    Ok(base.home_dir().join("Library").join("LaunchAgents"))
}

/// Returns the path to our plist file.
fn plist_path() -> Result<PathBuf, LaunchAgentError> {
    Ok(launch_agents_dir()?.join(PLIST_NAME))
}

/// Returns the path to the current executable.
fn exe_path() -> Result<PathBuf, LaunchAgentError> {
    env::current_exe().map_err(LaunchAgentError::NoExePath)
}

/// Generates the plist content for the LaunchAgent.
fn generate_plist(exe: &std::path::Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.pomobar</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
        exe.display()
    )
}

/// Enables launch at login by creating the LaunchAgent plist.
pub fn enable() -> Result<(), LaunchAgentError> {
    let dir = launch_agents_dir()?;
    let path = plist_path()?;
    let exe = exe_path()?;

    // Ensure the LaunchAgents directory exists
    fs::create_dir_all(&dir)?;

    // Write the plist file
    let content = generate_plist(&exe);
    fs::write(&path, content)?;

    Ok(())
}

/// Disables launch at login by removing the LaunchAgent plist.
pub fn disable() -> Result<(), LaunchAgentError> {
    let path = plist_path()?;

    if path.exists() {
        fs::remove_file(&path)?;
    }

    Ok(())
}

/// Sets the launch at login state.
pub fn set_enabled(enabled: bool) -> Result<(), LaunchAgentError> {
    if enabled {
        enable()
    } else {
        disable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plist_generation() {
        let exe = PathBuf::from("/usr/local/bin/pomobar");
        let plist = generate_plist(&exe);

        assert!(plist.contains("com.pomobar"));
        assert!(plist.contains("/usr/local/bin/pomobar"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
    }

    #[test]
    fn test_launch_agents_dir() {
        let dir = launch_agents_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.ends_with("Library/LaunchAgents"));
    }
}
