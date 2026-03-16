use std::process::{Command, Stdio};
use std::io::Write;
use anyhow::{Result, Context};
use bitwarden_core::Client;
use crate::vault::decrypt_string;
use crate::models::BwCipher;

pub fn execute_snippet(client: &Client, item: BwCipher) -> Result<()> {
    let enc_content = item.notes.as_ref()
        .context("Acest item nu are conținut în secțiunea 'Notes'")?;

    let content = decrypt_string(client, enc_content, item.organization_id.as_deref())?;

    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("Snippet-ul este gol."));
    }

    // 1. Identificăm pane-ul țintă
    // Dacă suntem într-un popup, "#{popup_pane_id}" va fi populat, și vrem să trimitem la pane-ul anterior ("!")
    // Altfel, trimitem la pane-ul curent (".").
    let target_pane = if let Ok(output) = Command::new("tmux").args(["display-message", "-p", "#{popup_pane_id}"]).output() {
        let popup_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !popup_id.is_empty() {
             "!".to_string() 
        } else {
             ".".to_string()
        }
    } else {
        "!".to_string()
    };

    let name = item.name.as_deref().unwrap_or("Snippet");
    println!("📝 Injectare snippet '{}'...", name);

    // 2. Încărcăm conținutul într-un buffer tmux pentru injecție multi-line robustă
    let normalized_content = content.replace("\r\n", "\n").replace('\r', "\n");
    
    let mut child = Command::new("tmux")
        .args(["load-buffer", "-b", "zbw_snippet", "-"])
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(normalized_content.as_bytes())?;
    }
    child.wait()?;

    // 3. Pastăm bufferul
    let status = Command::new("tmux")
        .args(["paste-buffer", "-b", "zbw_snippet", "-t", &target_pane])
        .status()?;

    if status.success() {
        // Ștergem bufferul temporar
        let _ = Command::new("tmux").args(["delete-buffer", "-b", "zbw_snippet"]).status();
        println!("✅ Snippet trimis.");
    }

    Ok(())
}
