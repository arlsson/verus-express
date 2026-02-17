//
// Clipboard command handlers
// Security: Reads plain text only and never logs clipboard contents

use crate::types::WalletError;

#[tauri::command(rename_all = "snake_case")]
pub async fn read_clipboard_text() -> Result<String, WalletError> {
    read_clipboard_text_impl()
}

fn read_stdout_from_command(command: &str, args: &[&str]) -> Result<String, WalletError> {
    let output = std::process::Command::new(command)
        .args(args)
        .output()
        .map_err(|_| WalletError::OperationFailed)?;
    if !output.status.success() {
        return Err(WalletError::OperationFailed);
    }
    String::from_utf8(output.stdout).map_err(|_| WalletError::OperationFailed)
}

#[cfg(target_os = "macos")]
fn read_clipboard_text_impl() -> Result<String, WalletError> {
    read_stdout_from_command("pbpaste", &[])
}

#[cfg(target_os = "windows")]
fn read_clipboard_text_impl() -> Result<String, WalletError> {
    read_stdout_from_command("powershell", &["-NoProfile", "-Command", "Get-Clipboard"])
}

#[cfg(target_os = "linux")]
fn read_clipboard_text_impl() -> Result<String, WalletError> {
    read_stdout_from_command("wl-paste", &["-n"])
        .or_else(|_| read_stdout_from_command("xclip", &["-selection", "clipboard", "-o"]))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn read_clipboard_text_impl() -> Result<String, WalletError> {
    Err(WalletError::OperationFailed)
}
