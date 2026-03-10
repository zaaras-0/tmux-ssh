use std::process::Command;
use anyhow::{Result, Context};
use crate::models::BwItem;

pub fn execute_snippet(item: BwItem) -> Result<()> {
    // În Bitwarden, snippet-urile le stocăm de obicei în câmpul 'notes'
    let content = item.notes.as_ref()
        .context("Acest item nu are conținut în secțiunea 'Notes'")?;

    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("Snippet-ul este gol."));
    }

    // Luăm ID-ul panoului curent
    let pane_id = std::env::var("TMUX_PANE")
        .context("Nu ești într-o sesiune Tmux. Nu pot injecta snippet-ul.")?;

    println!("📝 Injectare snippet: {}...", item.name);

    // Folosim 'send-keys -l' (literal). 
    // Acest flag este critic: face ca Tmux să NU interpreteze caracterele 
    // speciale (ca $, *, ", ') ci să le trimită ca și cum ar fi tastate de om.
    let status = Command::new("tmux")
        .args(["send-keys", "-t", &pane_id, "-l", content])
        .status()?;

    if status.success() {
        // Opțional: Trimitem un 'Enter' la final pentru a executa comanda imediat
        // Putem face asta configurabil în viitor.
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &pane_id, "Enter"])
            .status();
        
        println!("✅ Snippet trimis cu succes.");
    }

    Ok(())
}