use std::process::Command;
use std::os::unix::process::CommandExt;
use std::thread;
use std::time::Duration;
use anyhow::{Result, Context, anyhow};
use crate::models::BwItem;

pub fn execute_ssh(item: BwItem) -> Result<()> {
    let login = item.login.as_ref().context("Acest item nu are date de login (username/password)")?;
    let username = login.username.as_deref().unwrap_or("root");
    let password = login.password.as_deref().unwrap_or("");
    
    // Extragem primul URI valid (host-ul)
    let host = item.login.as_ref()
        .and_then(|l| l.uris.as_ref())
        .and_then(|u| u.first())
        .and_then(|uri| uri.uri.as_deref())
        .context("Item-ul nu are un URI valid pentru SSH")?;

    // Curățăm host-ul de prefixe tip ssh://
    let host_clean = host.strip_prefix("ssh://").unwrap_or(host).trim();

    // 1. Redenumim fereastra Tmux (UX)
    let _ = Command::new("tmux")
        .args(["rename-window", &format!("ssh:{}", item.name)])
        .status();

    // 2. Pregătim injectarea parolei într-un thread separat
    if !password.is_empty() {
        spawn_password_injector(password.to_string())?;
    }

    // 3. Executăm SSH - folosim .exec() care înlocuiește procesul zbw cu ssh
    // Astfel, când închizi SSH-ul, se închide și instanța curentă.
    println!("🚀 Conectare la {}...", host_clean);
    
    let err = Command::new("ssh")
        .arg(format!("{}@{}", username, host_clean))
        .exec(); // Această linie oprește execuția Rust dacă reușește

    // Dacă ajungem aici, ssh a eșuat să pornească
    Err(anyhow!("Eșec la pornirea SSH: {}", err))
}

fn spawn_password_injector(password: String) -> Result<()> {
    // Luăm ID-ul panoului curent din mediu
    let pane_id = std::env::var("TMUX_PANE")
        .context("Nu ești într-o sesiune Tmux. Injectarea parolei eșuată.")?;

    thread::spawn(move || {
        // Delay-ul necesar pentru ca SSH să inițieze handshaking-ul și să ceară parola
        // 1.5s este de obicei suficient, dar am putea să-l facem configurabil
        thread::sleep(Duration::from_millis(1500));

        // Injectăm parola literal (-l) pentru a evita interpretarea caracterelor speciale
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &pane_id, &password, "Enter"])
            .status();
        
        // Curățăm orice urmă de parolă din buffer-ul Tmux dacă e cazul (opțional)
    });

    Ok(())
}