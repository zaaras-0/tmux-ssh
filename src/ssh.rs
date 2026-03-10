use std::process::Command;
use std::os::unix::process::CommandExt;
use std::thread;
use std::time::Duration;
use anyhow::{Result, Context, anyhow};
use bitwarden_api_api::models::CipherDetailsResponseModel;
use bitwarden_core::Client;
use crate::vault::decrypt_string;

pub fn execute_ssh(client: &Client, item: CipherDetailsResponseModel) -> Result<()> {
    let name = item.name.as_deref().unwrap_or("Unknown");
    let login = item.login.as_ref().context("Acest item nu are date de login")?;
    
    let oid = item.organization_id.map(|u| u.to_string());
    let oid_ref = oid.as_deref();

    // Decriptăm username și password
    let username = if let Some(enc_user) = &login.username {
        decrypt_string(client, enc_user, oid_ref).unwrap_or_else(|_| "root".to_string())
    } else {
        "root".to_string()
    };

    let password = if let Some(enc_pass) = &login.password {
        decrypt_string(client, enc_pass, oid_ref).unwrap_or_default()
    } else {
        String::new()
    };
    
    // URI-urile nu sunt de obicei criptate la nivel de string individual în modelul ăsta?
    // Ba da, Bitwarden API returnează URI-urile ca obiecte cu câmp 'uri' criptat.
    let host = login.uris.as_ref()
        .and_then(|u| u.first())
        .and_then(|uri| uri.uri.as_ref())
        .and_then(|enc_uri| decrypt_string(client, enc_uri, oid_ref).ok())
        .context("Item-ul nu are un URI valid pentru SSH (sau decriptare eșuată)")?;

    let host_clean = host.strip_prefix("ssh://").unwrap_or(&host).trim();

    let _ = Command::new("tmux")
        .args(["rename-window", &format!("ssh:{}", name)])
        .status();

    if !password.is_empty() {
        spawn_password_injector(password);
    }

    println!("🚀 Conectare la {}...", host_clean);
    
    let err = Command::new("ssh")
        .arg(format!("{}@{}", username, host_clean))
        .exec();

    Err(anyhow!("Eșec la pornirea SSH: {}", err))
}

fn spawn_password_injector(password: String) {
    if let Ok(pane_id) = std::env::var("TMUX_PANE") {
        // Salvează parola în opțiunile pane-ului pentru tmux-insert-pass script (Prefix+p)
        let _ = Command::new("tmux")
            .args(["set-option", "-p", "-t", &pane_id, "@server_pass", &password])
            .status();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1500));
            let _ = Command::new("tmux")
                .args(["send-keys", "-t", &pane_id, &password, "Enter"])
                .status();
        });
    }
}
