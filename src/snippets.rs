use std::process::Command;
use anyhow::{Result, Context};
use bitwarden_api_api::models::CipherDetailsResponseModel;
use bitwarden_core::Client;
use crate::vault::decrypt_string;

pub fn execute_snippet(client: &Client, item: CipherDetailsResponseModel) -> Result<()> {
    let enc_content = item.notes.as_ref()
        .context("Acest item nu are conținut în secțiunea 'Notes'")?;

    let oid = item.organization_id.map(|u| u.to_string());
    let content = decrypt_string(client, enc_content, oid.as_deref())?;

    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("Snippet-ul este gol."));
    }

    // 1. Identificăm pane-ul țintă (cel de unde am venit)
    // În tmux, dacă am rulat zbw snippets într-un split, pane-ul anterior este de obicei 'last'
    let target_pane = if let Ok(output) = Command::new("tmux").args(["display-message", "-p", "#{last_pane_id}"]).output() {
        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if id.is_empty() { 
            // Fallback la pane-ul curent dacă nu există 'last'
            std::env::var("TMUX_PANE").unwrap_or_else(|_| "{last}".to_string())
        } else {
            id
        }
    } else {
        "{last}".to_string()
    };

    let name = item.name.as_deref().unwrap_or("Snippet");
    println!("📝 Injectare snippet '{}' în pane {}...", name, target_pane);

    // 2. Trimitem conținutul snippet-ului
    // Folosim -l pentru literal (nu interpretează taste speciale)
    let status = Command::new("tmux")
        .args(["send-keys", "-t", &target_pane, "-l", &content])
        .status()?;

    if status.success() {
        // 3. Trimitem 'Enter' pentru a executa comanda
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &target_pane, "Enter"])
            .status();
        
        println!("✅ Snippet trimis.");
        
        // 4. Închidem pane-ul curent (selectorul) pentru a reveni rapid la munca noastră
        if let Ok(current_pane) = std::env::var("TMUX_PANE") {
            let _ = Command::new("tmux")
                .args(["kill-pane", "-t", &current_pane])
                .status();
        }
    }

    Ok(())
}
