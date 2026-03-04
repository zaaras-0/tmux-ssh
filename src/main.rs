use skim::prelude::*;
use std::process::Command;
use std::os::unix::process::CommandExt;
use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone)]
struct Server {
    name: String,
    folder: String,
}

impl SkimItem for Server {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} [{}]", self.name, self.folder))
    }

    fn display(&self, _context: DisplayContext<'_>) -> AnsiString<'_> {
        AnsiString::parse(&format!("\x1b[32m{}\x1b[0m \x1b[90m[{}]\x1b[0m", self.name, self.folder))
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}

fn get_rbw_list() -> Result<Vec<Server>> {
    let output = Command::new("rbw")
        .arg("list")
        .arg("--fields")
        .arg("folder,name")
        .output()
        .context("Failed to execute rbw list")?;

    if !output.status.success() {
        // Try to unlock if failed
        println!("Bitwarden vault is locked. Unlocking...");
        let unlock_status = Command::new("rbw").arg("unlock").status()?;
        if !unlock_status.success() {
            return Err(anyhow!("Failed to unlock Bitwarden vault"));
        }
        // Retry after unlock
        return get_rbw_list();
    }

    let s = String::from_utf8_lossy(&output.stdout);
    let servers = s.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                Some(Server {
                    folder: parts[0].to_string(),
                    name: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .filter(|s| s.folder == "Servers")
        .collect();
    
    Ok(servers)
}

fn fuzzy_select(items: Vec<Server>) -> Option<Server> {
    let options = SkimOptionsBuilder::default()
        .height(Some("40%"))
        .reverse(true)
        .prompt(Some("🚀 Server: "))
        .color(Some("dark,fg:242,bg:236,hl:65,fg+:250,bg+:238,hl+:108,info:108,prompt:109,pointer:168,marker:168,spinner:108,header:108"))
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in items {
        let _ = tx_item.send(std::sync::Arc::new(item));
    }
    drop(tx_item);

    let selected_items = Skim::run_with(&options, Some(rx_item))
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);

    selected_items.first().map(|item| {
        (**item).as_any().downcast_ref::<Server>().unwrap().clone()
    })
}

fn get_server_details(name: &str) -> Result<(String, String, String)> {
    let output = Command::new("rbw")
        .arg("get")
        .arg("--full")
        .arg(name)
        .output()
        .context("Failed to get server details from rbw")?;

    if !output.status.success() {
        return Err(anyhow!("rbw get failed for {}", name));
    }

    let s = String::from_utf8_lossy(&output.stdout);
    let mut password = String::new();
    let mut username = String::new();
    let mut uri = String::new();

    let mut lines = s.lines();
    if let Some(first_line) = lines.next() {
        password = first_line.trim().to_string();
    }

    for line in lines {
        if line.starts_with("Username: ") {
            username = line.strip_prefix("Username: ").unwrap().trim().to_string();
        } else if line.starts_with("URI: ") {
            if uri.is_empty() {
                uri = line.strip_prefix("URI: ").unwrap().trim().to_string();
            }
        }
    }
    
    Ok((username, password, uri))
}

fn main() -> Result<()> {
    let servers = get_rbw_list()?;
    
    if servers.is_empty() {
        println!("❌ Error: No servers found in Bitwarden folder 'Servers'.");
        println!("Please ensure you have items in a folder named 'Servers'.");
        std::thread::sleep(std::time::Duration::from_secs(3));
        return Ok(());
    }

    let selection = match fuzzy_select(servers) {
        Some(s) => s,
        None => return Ok(()),
    };

    let (user, pass, uri) = get_server_details(&selection.name)?;

    if uri.is_empty() {
        return Err(anyhow!("No URI found for server {}", selection.name));
    }

    let host = uri.strip_prefix("ssh://").unwrap_or(&uri).trim();

    // Set the password in tmux pane-local variable
    let _ = Command::new("tmux")
        .arg("set-option")
        .arg("-p")
        .arg("@server_pass")
        .arg(&pass)
        .status();

    // Rename the current window to the server name
    let _ = Command::new("tmux")
        .arg("rename-window")
        .arg(&selection.name)
        .status();

    println!("Connecting to {} as {}...", host, user);

    // Replace current process with ssh
    let err = Command::new("ssh")
        .arg(format!("{}@{}", user, host))
        .exec();

    // If exec returns, it failed
    Err(anyhow!("Failed to execute ssh: {}", err))
}
