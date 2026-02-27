use std::process::{Command, Stdio};
use std::io::{Write};

fn main() {
    // 1. Verificăm dacă rbw este deblocat
    let unlocked = Command::new("rbw")
        .arg("unlocked")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !unlocked {
        // Dacă e blocat, cerem deblocarea direct în panou
        println!("🔐 Bitwarden is locked. Unlocking...");
        let _ = Command::new("rbw").arg("unlock").status();
    }

    // 2. Extragem lista de servere (doar numele/alias-urile)
    let output = Command::new("rbw").arg("list").output().expect("Failed to list");
    let input = String::from_utf8_lossy(&output.stdout);

    // 3. Interfața de selecție (Skim/FZF integrat)
    let mut child = Command::new("fzf") // Folosim fzf dacă e instalat, pt viteză și UI familiar
        .arg("--reverse")
        .arg("--height=40%")
        .arg("--prompt=🚀 Server: ")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Please install 'fzf' for the best experience.");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(input.as_bytes()).unwrap();
    }

    let result = child.wait_with_output().unwrap();
    let selection = String::from_utf8_lossy(&result.stdout).trim().to_string();

    if selection.is_empty() { return; }

    // 4. Extragem parola și user-ul (opțional)
    let pass = Command::new("rbw").arg("get").arg(&selection).output().unwrap();
    let password = String::from_utf8_lossy(&pass.stdout).trim().to_string();

    // 5. Trimitem comanda către Tmux pentru a crea mediul izolat
    let tmux_cmd = format!(
        "tmux new-window -n '{0}'; \
         tmux set-option -p -t '{0}' @server_pass '{1}'; \
         tmux send-keys -t '{0}' 'ssh {0}' Enter",
        selection, password
    );

    Command::new("sh").arg("-c").arg(tmux_cmd).spawn().unwrap();
}