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

    // 2. Citim parola. Încercăm întâi la nivel de pane, apoi la nivel de sesiune.
    let mut password = String::new();
    
    // Încercăm pane
    if let Ok(out) = Command::new("tmux").args(["show-options", "-pv", "-t", &pane_id, "@server_pass"]).output() {
        password = String::from_utf8_lossy(&out.stdout).trim().to_string();
    }

    // Dacă e gol, încercăm session
    if password.is_empty() {
        if let Ok(out) = Command::new("tmux").args(["show-options", "-sv", "@server_pass"]).output() {
            password = String::from_utf8_lossy(&out.stdout).trim().to_string();
            log_debug("Password found at session level");
        }
    }

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

/// Deschide o sesiune nouă de tmux care va rula comanda de conectare internă
pub fn spawn_ssh_session(item: &BwCipher, selected_uri: Option<String>) -> Result<()> {
    let raw_name = item.name.as_deref().unwrap_or("Unknown");
    // Igienizăm numele pentru tmux (fără puncte sau caractere speciale problematice)
    let session_name = raw_name.replace('.', "_").replace(':', "_").replace(' ', "_");
    
    let exe = std::env::current_exe()?.to_string_lossy().to_string();
    let id = &item.id;
    let ip = selected_uri.unwrap_or_default();
    let connect_cmd = format!("{} _connect {} \"{}\"", exe, id, ip);

    println!("🪟 Se deschide sesiunea nouă '{}' pentru {}...", session_name, raw_name);

    // 1. Creăm sesiunea cu default-command setat
    // Folosim -d pentru a nu face attach imediat, deoarece vrem să setăm opțiuni întâi dacă e cazul
    // sau să folosim switch-client dacă suntem deja în tmux.
    let status = Command::new("tmux")
        .args([
            "new-session", 
            "-d",
            "-s", &session_name,
            "-n", "ssh",
            &connect_cmd
        ])
        .status()?;

    if !status.success() {
        // Dacă sesiunea există deja, poate ar trebui să facem doar switch? 
        // User-ul a cerut sesiune nouă, dar în tmux numele trebuie să fie unic.
        println!("⚠️ Sesiunea '{}' există deja sau nu a putut fi creată.", session_name);
    }

    // 2. Setăm default-command pentru ferestre/pane-uri noi în această sesiune
    let _ = Command::new("tmux")
        .args(["set-option", "-t", &session_name, "default-command", &connect_cmd])
        .status();

    // 3. Personalizăm Status Bar-ul pentru această sesiune
    // Afișăm Numele Serverului și IP-ul în stânga
    let status_left = format!(
        "#[fg=black,bg=green,bold] #S #[fg=green,bg=black,nobold] #[fg=white,bold]{} #[fg=yellow,nobold] {} #[fg=black,bg=default,nobold] ",
        raw_name, ip
    );
    let _ = Command::new("tmux")
        .args(["set-option", "-t", &session_name, "status-left", &status_left])
        .status();
    let _ = Command::new("tmux")
        .args(["set-option", "-t", &session_name, "status-left-length", "100"])
        .status();

    // 4. Facem switch la sesiune
    if std::env::var("TMUX").is_ok() {
        Command::new("tmux").args(["switch-client", "-t", &session_name]).status()?;
    } else {
        Command::new("tmux").args(["attach-session", "-t", &session_name]).status()?;
    }

    Ok(())
}

/// Execută conectarea SSH efectivă
pub async fn execute_ssh_internal(config: &Config, id: &str, selected_ip: Option<String>) -> Result<()> {
    let mut client = crate::auth::get_client(config).await?;
    let details = get_server_details(config, &mut client, id, selected_ip).await?;

    set_tmux_metadata(&details.name, details.password.as_deref().unwrap_or(""), &details.host);

    if let Some(password) = &details.password {
        spawn_password_injector(password);
    }

    println!("🚀 Conectare la {} ({} : port {})...", details.name, details.host, details.port);
    
    let err = Command::new("ssh")
        .arg("-p")
        .arg(details.port.to_string())
        .arg(format!("{}@{}", details.username, details.host))
        .exec();

    Err(anyhow!("Eșec la pornirea SSH: {}", err))
}

pub async fn get_server_details(
    config: &Config, 
    client: &mut bitwarden_core::Client, 
    id: &str, 
    selected_ip: Option<String>
) -> Result<crate::models::ServerDetails> {
    let items = crate::vault::fetch_filtered_items(config, client, false, false).await?;

    let item = items.into_iter().find(|i| i.id == id)
        .context("Serverul nu a mai fost găsit în vault (ID invalid)")?;

    let name = item.name.as_deref().unwrap_or("Unknown").to_string();
    let login = item.login.as_ref().context("Acest item nu are date de login")?;
    let oid_ref = item.organization_id.as_deref();

    let username = login.username.as_ref()
        .and_then(|enc| decrypt_string(client, enc, oid_ref).ok())
        .unwrap_or_else(|| "root".to_string());

    let password = login.password.as_ref()
        .and_then(|enc| decrypt_string(client, enc, oid_ref).ok());
    
    let host = if let Some(ip) = selected_ip {
        if ip.is_empty() {
            return Err(anyhow!("IP-ul selectat este gol."));
        }
        ip
    } else {
        login.uris.as_ref()
            .and_then(|u| u.first())
            .and_then(|uri| uri.uri.as_ref())
            .and_then(|enc_uri| decrypt_string(client, enc_uri, oid_ref).ok())
            .context("Item-ul nu are un URI valid pentru SSH")?
    };

    let host_clean = host.strip_prefix("ssh://").unwrap_or(&host).trim().to_string();

    // Căutăm un câmp custom pentru PORT
    let mut port = 22;
    if let Some(fields) = &item.fields {
        for f in fields {
            if let (Some(fname), Some(fval)) = (&f.name, &f.value) {
                let dec_fname = decrypt_string(client, fname, oid_ref).unwrap_or(fname.clone());
                if dec_fname.to_lowercase() == "port" {
                    let p_str = decrypt_string(client, fval, oid_ref).unwrap_or(fval.clone());
                    port = p_str.parse().unwrap_or(22);
                    break;
                }
            }
        }
    }

    Ok(crate::models::ServerDetails {
        name,
        host: host_clean,
        port,
        username,
        password,
    })
}

fn set_tmux_metadata(name: &str, pass: &str, ip: &str) {
    if let Ok(pane_id) = std::env::var("TMUX_PANE") {
        // La nivel de Pane
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_name", name]).status();
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_pass", pass]).status();
        let _ = Command::new("tmux").args(["set-option", "-p", "-t", &pane_id, "@server_ip", ip]).status();
        
        // La nivel de Sesiune (pentru pane-uri noi)
        let _ = Command::new("tmux").args(["set-option", "@server_name", name]).status();
        let _ = Command::new("tmux").args(["set-option", "@server_pass", pass]).status();
        let _ = Command::new("tmux").args(["set-option", "@server_ip", ip]).status();

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
