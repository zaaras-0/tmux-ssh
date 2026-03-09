use skim::prelude::*;
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use rayon::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RbwItem {
    id: String,
    name: String,
    folder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RbwItemFull {
    id: String,
    name: String,
    folder: Option<String>,
    organization: Option<String>,
    notes: Option<String>,
    data: Option<RbwItemData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RbwItemData {
    username: Option<String>,
    password: Option<String>,
    uris: Option<Vec<RbwUri>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RbwUri {
    uri: String,
}

#[derive(Debug, Clone)]
struct Server {
    id: String,
    name: String,
    group: String,
    user: String,
    pass: String,
    uris: Vec<String>,
    notes: Option<String>,
}

impl SkimItem for Server {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} {}", self.group, self.name))
    }

    fn display(&self, _context: DisplayContext<'_>) -> AnsiString<'_> {
        let group_color = if self.group == "Personal" { "\x1b[34m" } else { "\x1b[35m" };
        AnsiString::parse(&format!("{}[{}]\x1b[0m \x1b[32m{}\x1b[0m", group_color, self.group, self.name))
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.id)
    }
}

#[derive(Debug, Clone)]
struct Snippet {
    id: String,
    name: String,
    group: String,
    notes: String,
}

impl SkimItem for Snippet {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} {}", self.group, self.name))
    }

    fn display(&self, _context: DisplayContext<'_>) -> AnsiString<'_> {
        let group_color = if self.group == "Personal" { "\x1b[34m" } else { "\x1b[35m" };
        AnsiString::parse(&format!("{}[{}]\x1b[0m \x1b[33m📜 {}\x1b[0m", group_color, self.group, self.name))
    }
}

#[derive(Debug, Clone)]
struct UriItem {
    uri: String,
}

impl SkimItem for UriItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.uri)
    }

    fn display(&self, _context: DisplayContext<'_>) -> AnsiString<'_> {
        AnsiString::parse(&format!("\x1b[36m{}\x1b[0m", self.uri))
    }
}

fn get_rbw_items(folder_name: &str) -> Result<Vec<RbwItem>> {
    // Check if rbw is configured by checking if an email is set
    let config_check = Command::new("rbw")
        .arg("config")
        .arg("show")
        .output()
        .context("Failed to check rbw config")?;

    let config_output = String::from_utf8_lossy(&config_check.stdout);
    if !config_output.contains("email:") || config_output.contains("email: <not set>") {
        println!("🚀 Bitwarden (rbw) is not configured.");
        println!("Please enter your Bitwarden email:");
        let mut email = String::new();
        std::io::stdin().read_line(&mut email)?;
        let email = email.trim();

        if !email.is_empty() {
            Command::new("rbw").arg("config").arg("set").arg("email").arg(email).status()?;
            println!("✅ Email set to {}. Now logging in...", email);
            Command::new("rbw").arg("login").status()?;
        } else {
            return Err(anyhow!("Email is required to configure rbw."));
        }
    }

    let output = Command::new("rbw")
        .arg("list")
        .arg("--raw")
        .output()
        .context("Failed to execute rbw list --raw")?;

    if !output.status.success() {
        println!("🔓 Bitwarden vault is locked. Unlocking...");
        let unlock_status = Command::new("rbw").arg("unlock").status()?;
        if !unlock_status.success() {
            return Err(anyhow!("Failed to unlock Bitwarden vault"));
        }
        return get_rbw_items(folder_name);
    }
...
    let items: Vec<RbwItem> = serde_json::from_slice(&output.stdout)
        .context("Failed to parse rbw list output")?;

    Ok(items.into_iter()
        .filter(|item| item.folder.as_deref() == Some(folder_name))
        .collect())
}

fn get_server_details(filtered_items: Vec<RbwItem>) -> Vec<Server> {
    filtered_items.into_par_iter()
        .filter_map(|item| {
            let detail_output = Command::new("rbw")
                .arg("get")
                .arg(&item.id)
                .arg("--raw")
                .output()
                .ok()?;

            if !detail_output.status.success() {
                return None;
            }

            let full: RbwItemFull = serde_json::from_slice(&detail_output.stdout).ok()?;
            
            let group = full.organization.unwrap_or_else(|| "Personal".to_string());
            let (user, pass, uris) = if let Some(d) = full.data {
                let u = d.username.unwrap_or_default();
                let p = d.password.unwrap_or_default();
                let uris = d.uris.unwrap_or_default().into_iter().map(|ru| ru.uri).collect();
                (u, p, uris)
            } else {
                (String::new(), String::new(), Vec::new())
            };

            Some(Server {
                id: full.id,
                name: full.name,
                group,
                user,
                pass,
                uris,
                notes: full.notes,
            })
        })
        .collect()
}

fn get_snippet_details(filtered_items: Vec<RbwItem>) -> Vec<Snippet> {
    filtered_items.into_par_iter()
        .filter_map(|item| {
            let detail_output = Command::new("rbw")
                .arg("get")
                .arg(&item.id)
                .arg("--raw")
                .output()
                .ok()?;

            if !detail_output.status.success() {
                return None;
            }

            let full: RbwItemFull = serde_json::from_slice(&detail_output.stdout).ok()?;
            let notes = full.notes.unwrap_or_default();

            if notes.is_empty() {
                return None;
            }

            let group = full.organization.unwrap_or_else(|| "Personal".to_string());

            Some(Snippet {
                id: full.id,
                name: full.name,
                group,
                notes,
            })
        })
        .collect()
}

fn fuzzy_select<T: SkimItem + Clone + 'static>(items: Vec<T>, prompt: &str) -> Option<T> {
    let options = SkimOptionsBuilder::default()
        .height(Some("40%"))
        .reverse(true)
        .prompt(Some(prompt))
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

    selected_items.first().and_then(|item| {
        (**item).as_any().downcast_ref::<T>().cloned()
    })
}

fn run_snippets() -> Result<()> {
    let items = get_rbw_items("Snippets")?;
    if items.is_empty() {
        println!("❌ Error: No snippets found in folder 'Snippets'.");
        std::thread::sleep(std::time::Duration::from_secs(3));
        return Ok(());
    }

    let mut snippets = get_snippet_details(items);
    
    // Sort items: Personal first, then others alphabetically by group, then by name
    snippets.sort_by(|a, b| {
        if a.group == b.group {
            a.name.cmp(&b.name)
        } else if a.group == "Personal" {
            std::cmp::Ordering::Less
        } else if b.group == "Personal" {
            std::cmp::Ordering::Greater
        } else {
            a.group.cmp(&b.group)
        }
    });

    if let Some(selection) = fuzzy_select(snippets, "📜 Snippet: ") {
        // Send literal content directly to the pane
        // -l sends keys literally, avoiding tmux parsing
        let _ = Command::new("tmux")
            .arg("send-keys")
            .arg("-l")
            .arg(&selection.notes)
            .status();

        // Optional: Send Enter if you want snippets to execute immediately
        // Usually safer to just paste, but can be configured.
        let _ = Command::new("tmux")
            .arg("send-keys")
            .arg("Enter")
            .status();
    }
    Ok(())
}

fn run_servers() -> Result<()> {
    let items = get_rbw_items("Servers")?;
    if items.is_empty() {
        println!("❌ Error: No servers found in Bitwarden folder 'Servers'.");
        std::thread::sleep(std::time::Duration::from_secs(3));
        return Ok(());
    }

    let mut servers = get_server_details(items);
    
    // Sort items: Personal first, then others alphabetically by group, then by name
    servers.sort_by(|a, b| {
        if a.group == b.group {
            a.name.cmp(&b.name)
        } else if a.group == "Personal" {
            std::cmp::Ordering::Less
        } else if b.group == "Personal" {
            std::cmp::Ordering::Greater
        } else {
            a.group.cmp(&b.group)
        }
    });

    let selection = match fuzzy_select(servers, "🚀 Server: ") {
        Some(s) => s,
        None => return Ok(()),
    };

    let selected_uri = if selection.uris.is_empty() {
        return Err(anyhow!("No URI found for server {}", selection.name));
    } else if selection.uris.len() == 1 {
        selection.uris[0].clone()
    } else {
        match fuzzy_select(selection.uris.into_iter().map(|u| UriItem { uri: u }).collect(), "🌐 Select IP: ") {
            Some(u) => u.uri,
            None => return Ok(()),
        }
    };

    let host = selected_uri.strip_prefix("ssh://").unwrap_or(&selected_uri).trim();
    let current_pane_id = std::env::var("TMUX_PANE").context("Not running in tmux?")?;

    // Set the password in tmux pane-local variable
    let _ = Command::new("tmux")
        .arg("set-option")
        .arg("-p")
        .arg("@server_pass")
        .arg(&selection.pass)
        .status();

    // Rename the current window to the server name
    let _ = Command::new("tmux")
        .arg("rename-window")
        .arg(&selection.name)
        .status();

    // Spawn a background process to auto-insert password after a delay
    let _ = Command::new("bash")
        .arg("-c")
        .arg(format!("sleep 1.5; PASS=$(tmux show-options -t {0} -pv @server_pass); [ -n \"$PASS\" ] && tmux send-keys -t {0} \"$PASS\" Enter", current_pane_id))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    println!("Connecting to {} as {}...", host, selection.user);

    // Replace current process with ssh
    let err = Command::new("ssh")
        .arg(format!("{}@{}", selection.user, host))
        .exec();

    // If exec returns, it failed
    Err(anyhow!("Failed to execute ssh: {}", err))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--snippets" {
        run_snippets()
    } else {
        run_servers()
    }
}
