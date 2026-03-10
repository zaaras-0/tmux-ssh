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

    let pane_id = std::env::var("TMUX_PANE")
        .context("Nu ești într-o sesiune Tmux.")?;

    let name = item.name.as_deref().unwrap_or("Snippet");
    println!("📝 Injectare snippet: {}...", name);

    let status = Command::new("tmux")
        .args(["send-keys", "-t", &pane_id, "-l", &content])
        .status()?;

    if status.success() {
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &pane_id, "Enter"])
            .status();
        println!("✅ Snippet trimis.");
    }

    Ok(())
}
