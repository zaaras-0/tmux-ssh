use std::process::Command;
use std::os::unix::process::CommandExt;
use anyhow::{Result, Context, anyhow};
use crate::vault::decrypt_string;
use crate::models::{BwCipher, Config};
use std::fs::OpenOptions;
use std::io::Write;

fn log_debug(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/zbw_pass.log") {
        let _ = writeln!(file, "{}", msg);
    }
}

/// Citeste parola din optiunile tmux ale pane-ului curent si o injectează
pub fn inject_password_from_tmux() -> Result<()> {
    log_debug("--- Pass command started ---");

    // 1. Obținem ID-ul pane-ului curent
    let pane_id = match std::env::var("TMUX_PANE") {
        Ok(id) => id,
        Err(_) => {
            log_debug("TMUX_PANE env missing, trying fallback via tmux display-message...");
            let out_res = Command::new("tmux")
                .args(["display-message", "-p", "#{pane_id}"])
                .output();
            
            match out_res {
                Ok(out) if out.status.success() => {
                    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    log_debug(&format!("Fallback found Pane ID: {}", id));
                    id
                },
                _ => {
                    log_debug("Error: Could not determine TMUX_PANE even with fallback");
                    return Ok(());
                }
            }
        }
    };

    log_debug(&format!("Pane ID: {}", pane_id));

    // 2. Citim parola. Revenim la show-options -pv care e mai testat pentru variabile de tip pane.
    let output_res = Command::new("tmux")
        .args(["show-options", "-pv", "-t", &pane_id, "@server_pass"])
        .output();

    let password = match output_res {
        Ok(out) if out.status.success() => {
            let pass = String::from_utf8_lossy(&out.stdout).trim().to_string();
            log_debug(&format!("Password found (length: {})", pass.len()));
            pass
        },
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            log_debug(&format!("Tmux show-options failed: {}", err));
            String::new()
        },
        Err(e) => {
            log_debug(&format!("Failed to execute tmux command: {}", e));
            String::new()
        }
    };

    if password.is_empty() {
        log_debug("No password found in @server_pass");
        let _ = Command::new("tmux")
            .args(["display-message", "-t", &pane_id, "❌ No password found for this pane"])
            .status();
        return Ok(());
    }

    // 3. Trimitem parola LITERAL
    log_debug("Sending keys via tmux...");
    
    let status_literal = Command::new("tmux")
        .args(["send-keys", "-t", &pane_id, "-l", "--", &password])
        .status();

    let status_enter = Command::new("tmux")
        .args(["send-keys", "-t", &pane_id, "Enter"])
        .status();

    if status_literal.map_or(false, |s| s.success()) && status_enter.map_or(false, |s| s.success()) {
        log_debug("Success: Keys sent");
        let _ = Command::new("tmux")
            .args(["display-message", "-t", &pane_id, "🔐 Password injected"])
            .status();
    } else {
        log_debug("Error: Failed to send keys via tmux");
        let _ = Command::new("tmux")
            .args(["display-message", "-t", &pane_id, "❌ Error sending keys"])
            .status();
    }

    Ok(())
}

/// Deschide o fereastră nouă de tmux care va rula comanda de conectare internă
pub fn spawn_ssh_window(item: &BwCipher, selected_uri: Option<String>) -> Result<()> {
    let name = item.name.as_deref().unwrap_or("Unknown");
    let exe = std::env::current_exe()?.to_string_lossy().to_string();
    let id = &item.id;
    let ip = selected_uri.unwrap_or_default();

    println!("🪟 Se deschide fereastra nouă pentru {}...", name);

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

/// Execută conectarea SSH efectivă
pub async fn execute_ssh_internal(config: &Config, id: &str, selected_ip: Option<String>) -> Result<()> {
    let mut client = crate::auth::get_client(config).await?;
    
    let items = crate::vault::fetch_filtered_items(config, &mut client, false).await?;
    let item = items.into_iter().find(|i| i.id == id)
        .context("Serverul nu a mai fost găsit în vault (ID invalid)")?;

    let name = item.name.as_deref().unwrap_or("Unknown");
    let login = item.login.as_ref().context("Acest item nu are date de login")?;
    let oid_ref = item.organization_id.as_deref();

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

    set_tmux_metadata(name, &password, host_clean);

    if !password.is_empty() {
        spawn_password_injector(&password);
    }

    println!("🚀 Conectare la {} ({})...", name, host_clean);
    
    let err = Command::new("ssh")
        .arg(format!("{}@{}", username, host_clean))
        .exec();

    Err(anyhow!("Eșec la pornirea SSH: {}", err))
}

fn set_tmux_metadata(name: &str, pass: &str, ip: &str) {
    if let Ok(pane_id) = std::env::var("TMUX_PANE") {
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_name", name]).status();
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_pass", pass]).status();
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_ip", ip]).status();
        
        let _ = Command::new("tmux").args(["set-environment", "SERVER_NAME", name]).status();
        let _ = Command::new("tmux").args(["set-environment", "SERVER_PASS", pass]).status();
        let _ = Command::new("tmux").args(["set-environment", "SERVER_IP", ip]).status();
    }
}

fn spawn_password_injector(password: &str) {
    if let Ok(pane_id) = std::env::var("TMUX_PANE") {
        let _ = Command::new("sh")
            .arg("-c")
            .arg("sleep 1.2; tmux send-keys -t \"$PANE_ID\" -l -- \"$PASS\"; tmux send-keys -t \"$PANE_ID\" Enter")
            .env("PANE_ID", pane_id)
            .env("PASS", password)
            .spawn();
    }
}
