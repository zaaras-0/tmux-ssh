use std::process::Command;
use std::os::unix::process::CommandExt;
use std::thread;
use std::time::Duration;
use anyhow::{Result, Context, anyhow};
use bitwarden_core::Client;
use crate::vault::decrypt_string;
use crate::models::{BwCipher, Config};

/// Deschide o fereastră nouă de tmux care va rula comanda de conectare internă
pub fn spawn_ssh_window(item: &BwCipher, selected_uri: Option<String>) -> Result<()> {
    let name = item.name.as_deref().unwrap_or("Unknown");
    let exe = std::env::current_exe()?.to_string_lossy().to_string();
    let id = &item.id;
    let ip = selected_uri.unwrap_or_default();

    println!("🪟 Se deschide fereastra nouă pentru {}...", name);

    // Folosim 'tmux new-window' pentru a lansa un nou proces zbw care va face SSH-ul efectiv
    let status = Command::new("tmux")
        .args([
            "new-window", 
            "-n", &format!("ssh:{}", name),
            &format!("{} _connect {} \"{}\"", exe, id, ip)
        ])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Eșec la crearea ferestrei tmux noi."));
    }

    Ok(())
}

/// Execută conectarea SSH efectivă (trebuie rulat în fereastra destinație)
pub async fn execute_ssh_internal(config: &Config, id: &str, selected_ip: Option<String>) -> Result<()> {
    let mut client = crate::auth::get_client(config).await?;
    
    // Avem nevoie de item pentru credențiale
    let items = crate::vault::fetch_filtered_items(config, &mut client, false).await?;
    let item = items.into_iter().find(|i| i.id == id)
        .context("Serverul nu a mai fost găsit în vault (ID invalid)")?;

    let name = item.name.as_deref().unwrap_or("Unknown");
    let login = item.login.as_ref().context("Acest item nu are date de login")?;
    let oid_ref = item.organization_id.as_deref();

    // Decriptăm tot ce ne trebuie
    let username = login.username.as_ref()
        .and_then(|enc| decrypt_string(&client, enc, oid_ref).ok())
        .unwrap_or_else(|| "root".to_string());

    let password = login.password.as_ref()
        .and_then(|enc| decrypt_string(&client, enc, oid_ref).ok())
        .unwrap_or_default();
    
    let host = if let Some(ip) = selected_ip {
        if ip.is_empty() {
            return Err(anyhow!("IP-ul selectat este gol."));
        }
        ip
    } else {
        login.uris.as_ref()
            .and_then(|u| u.first())
            .and_then(|uri| uri.uri.as_ref())
            .and_then(|enc_uri| decrypt_string(&client, enc_uri, oid_ref).ok())
            .context("Item-ul nu are un URI valid pentru SSH")?
    };

    let host_clean = host.strip_prefix("ssh://").unwrap_or(&host).trim();

    // Salvează metadatele în tmux environment/options
    set_tmux_metadata(name, &password, host_clean);

    if !password.is_empty() {
        spawn_password_injector(password);
    }

    println!("🚀 Conectare la {} ({})...", name, host_clean);
    
    let err = Command::new("ssh")
        .arg(format!("{}@{}", username, host_clean))
        .exec();

    Err(anyhow!("Eșec la pornirea SSH: {}", err))
}

fn set_tmux_metadata(name: &str, pass: &str, ip: &str) {
    if let Ok(pane_id) = std::env::var("TMUX_PANE") {
        // Setăm variabile de tip Option (Prefix + p le folosește pe acestea)
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_name", name]).status();
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_pass", pass]).status();
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_ip", ip]).status();
        
        // Setăm și variabile de mediu efective în sesiune (pentru scripturi shell)
        let _ = Command::new("tmux").args(["set-environment", "SERVER_NAME", name]).status();
        let _ = Command::new("tmux").args(["set-environment", "SERVER_PASS", pass]).status();
        let _ = Command::new("tmux").args(["set-environment", "SERVER_IP", ip]).status();
    }
}

fn spawn_password_injector(password: String) {
    if let Ok(pane_id) = std::env::var("TMUX_PANE") {
        thread::spawn(move || {
            // Așteptăm să se deschidă promptul de parolă al SSH-ului
            thread::sleep(Duration::from_millis(1200));
            let _ = Command::new("tmux")
                .args(["send-keys", "-t", &pane_id, &password, "Enter"])
                .status();
        });
    }
}
