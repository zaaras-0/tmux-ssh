use std::process::Command;
use std::fs;
use std::io::Write;
use anyhow::{Result, anyhow, Context};
use crate::prompts;
use bitwarden::{
    auth::login::PasswordLoginRequest,
    client::client_settings::{ClientSettings, DeviceType},
    BitwardenClient,
};

const SESSION_PATH: &str = "/dev/shm/zbw.session";

pub async fn login_native(email: &str, server_url: &str) -> Result<BitwardenClient> {
    // Configurare setări client
    let mut settings = ClientSettings::default();
    settings.base_url = server_url.to_string();
    settings.device_type = DeviceType::Linux;

    let mut bw_client = BitwardenClient::new(Some(settings));

    // Cerem parola prin prompt-ul nostru existent
    let password = crate::prompts::ask_password("Master Password")?;

    let login_request = PasswordLoginRequest {
        email: email.to_string(),
        password,
        ..Default::default()
    };

    // Executăm login-ul nativ
    bw_client.auth().login_password(&login_request).await
        .map_err(|e| anyhow!("Eroare Login SDK: {:?}", e))?;

    Ok(bw_client)
}

/// Setează serverul Bitwarden (ex: self-hosted sau cloud)
pub fn set_bw_server(url: &str) -> Result<()> {
    let status = Command::new("bw")
        .args(["config", "server", url])
        .status()
        .context("Eșec la rularea 'bw config server'")?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Bitwarden CLI nu a putut seta serverul URL."))
    }
}

/// Procesul de login interactiv folosit în wizard sau la nevoie
pub fn login_wizard(email: &str) -> Result<String> {
    println!("🔑 Autentificare pentru: {}", email);
    
    // Încercăm login. Notă: 'bw login' interactiv cere parola și 2FA
    let output = Command::new("bw")
        .args(["login", email, "--raw"])
        .output()
        .context("Eșec la pornirea procesului 'bw login'")?;

    if output.status.success() {
        let session = String::from_utf8(output.stdout)?.trim().to_string();
        save_session(&session)?;
        Ok(session)
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("Already logged in") {
            // Dacă suntem deja logați, dar seiful e blocat, rulăm unlock
            return unlock_vault();
        }
        Err(anyhow!("Login eșuat: {}", err))
    }
}

/// Deblochează seiful (cere Master Password)
pub fn unlock_vault() -> Result<String> {
    let password = crate::prompts::ask_password("Master Password")?;
    
    // Aici putem folosi .output() pentru că NU lăsăm 'bw' să ceară parola,
    // ci i-o dăm noi direct ca argument.
    let output = Command::new("bw")
        .args(["unlock", &password, "--raw"])
        .output()
        .context("Eșec la rularea 'bw unlock'")?;

    if output.status.success() {
        let session = String::from_utf8(output.stdout)?.trim().to_string();
        save_session(&session)?;
        Ok(session)
    } else {
        Err(anyhow!("Parolă incorectă sau eroare la deblocare."))
    }
}

/// Salvează sesiunea în RAM (/dev/shm) cu permisiuni restrictive
fn save_session(session: &str) -> Result<()> {
    let mut file = fs::File::create(SESSION_PATH)?;
    
    // Setăm permisiuni 600 (doar owner-ul citește/scrie)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        file.set_permissions(perms)?;
    }

    file.write_all(session.as_bytes())?;
    Ok(())
}

/// Încarcă sesiunea din RAM
pub fn get_active_session() -> Result<String> {
    if std::path::Path::new(SESSION_PATH).exists() {
        let session = fs::read_to_string(SESSION_PATH)?.trim().to_string();
        if !session.is_empty() {
            return Ok(session);
        }
    }
    
    // Dacă nu există sau e goală, încercăm să deblocăm
    unlock_vault()
}

/// Verifică statusul sesiunii (Logged out, Locked, Unlocked)
pub fn check_status(session: &Option<String>) -> Result<serde_json::Value> {
    let mut cmd = Command::new("bw");
    cmd.arg("status");
    
    if let Some(s) = session {
        cmd.args(["--session", s]);
    }

    let output = cmd.output()?;
    let status: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(status)
}

/// Șterge sesiunea din RAM (Log out / Lock)
pub fn purge_session() -> Result<()> {
    if std::path::Path::new(SESSION_PATH).exists() {
        fs::remove_file(SESSION_PATH)?;
    }
    Command::new("bw").arg("lock").status()?;
    println!("🔐 Sesiune eliminată din RAM și vault blocat.");
    Ok(())
}