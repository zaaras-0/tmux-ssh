use std::process::Command;
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
    // Când rulăm într-un popup, de obicei vrem să injectăm în pane-ul care era activ înainte.
    // 'last-pane' în contextul popup-ului este cel de sub el.
    let target_pane = if let Ok(output) = Command::new("tmux").args(["display-message", "-p", "#{pane_id}"]).output() {
        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Dacă suntem într-un popup, 'pane_id' ar trebui să fie pane-ul care a deschis popup-ul? 
        // Testele arată că uneori e nevoie de 'last'.
        id
    } else {
        "{last}".to_string()
    };

    let name = item.name.as_deref().unwrap_or("Snippet");
    println!("📝 Injectare snippet '{}' în pane {}...", name, target_pane);

    // 2. Trimitem conținutul snippet-ului
    let status = Command::new("tmux")
        .args(["send-keys", "-t", &target_pane, "-l", &content])
        .status()?;

    if status.success() {
        // 3. Trimitem 'Enter'
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &target_pane, "Enter"])
            .status();
        
        println!("✅ Snippet trimis.");
    }

    Ok(())
}
