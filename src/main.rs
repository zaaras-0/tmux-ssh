use skim::prelude::*;
use std::process::{Command};
use std::os::unix::process::CommandExt;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use rayon::prelude::*;
use std::borrow::Cow;

// --- Structuri Date ---
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
}

impl SkimItem for Server {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} {}", self.group, self.name))
    }
    fn display(&self, _context: DisplayContext<'_>) -> AnsiString<'_> {
        let group_color = if self.group == "Personal" { "\x1b[34m" } else { "\x1b[35m" };
        AnsiString::parse(&format!("{}[{}]\x1b[0m \x1b[32m{}\x1b[0m", group_color, self.group, self.name))
    }
    fn output(&self) -> Cow<'_, str> { Cow::Borrowed(&self.id) }
}

#[derive(Debug, Clone)]
struct Snippet {
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
struct UriItem { uri: String }
impl SkimItem for UriItem {
    fn text(&self) -> Cow<'_, str> { Cow::Borrowed(&self.uri) }
    fn display(&self, _context: DisplayContext<'_>) -> AnsiString<'_> {
        AnsiString::parse(&format!("\x1b[36m{}\x1b[0m", self.uri))
    }
}

// --- Logica de Core ---

fn ensure_rbw_ready() -> Result<()> {
    let check = Command::new("rbw").arg("unlocked").output();
    match check {
        Ok(output) if output.status.success() => Ok(()),
        _ => {
            println!("🔐 Bitwarden session inactive. Authenticating...");
            let status = Command::new("rbw").arg("unlock").status()
                .context("Failed to run rbw unlock")?;
            if status.success() { Ok(()) } 
            else { Err(anyhow!("Could not unlock rbw.")) }
        }
    }
}

fn fuzzy_select<T: SkimItem + Clone + 'static>(items: Vec<T>, prompt: &str) -> Result<T> {
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

    let output = Skim::run_with(&options, Some(rx_item))
        .ok_or_else(|| anyhow!("Skim execution error"))?;

    // Dacă utilizatorul apasă Ctrl+C sau ESC
    if output.is_abort {
        return Err(anyhow!("CANCELLED"));
    }

    output.selected_items.first()
        .and_then(|item| (**item).as_any().downcast_ref::<T>().cloned())
        .ok_or_else(|| anyhow!("No selection made"))
}

// --- Task-uri Scalabile ---

fn run_snippets() -> Result<()> {
    let raw_items = get_rbw_items("Snippets")?;
    let mut snippets = get_snippet_details(raw_items);
    
    // Sortare (Personal first...)
    sort_by_group_and_name(&mut snippets, |s| &s.group, |s| &s.name);

    let selection = fuzzy_select(snippets, "📜 Snippet: ")?; // Folosim ? pentru propagarea erorii (inclusiv CANCELLED)

    Command::new("tmux").args(["send-keys", "-l", &selection.notes]).status()?;
    Command::new("tmux").arg("send-keys").arg("Enter").status()?;
    
    Ok(())
}

fn run_servers() -> Result<()> {
    let raw_items = get_rbw_items("Servers")?;
    let mut servers = get_server_details(raw_items);
    
    sort_by_group_and_name(&mut servers, |s| &s.group, |s| &s.name);

    let selection = fuzzy_select(servers, "🚀 Server: ")?;

    let selected_uri = if selection.uris.len() > 1 {
        let uri_items = selection.uris.into_iter().map(|u| UriItem { uri: u }).collect();
        fuzzy_select(uri_items, "🌐 Select IP: ")?.uri
    } else {
        selection.uris.first().cloned().ok_or_else(|| anyhow!("No URI"))?
    };

    let host = selected_uri.strip_prefix("ssh://").unwrap_or(&selected_uri).trim().to_string();
    let current_pane_id = std::env::var("TMUX_PANE").context("Not in tmux")?;

    // Tmux logic
    Command::new("tmux").args(["set-option", "-p", "@server_pass", &selection.pass]).status()?;
    Command::new("tmux").args(["rename-window", &selection.name]).status()?;

    let cmd = format!("sleep 1.2; PASS=$(tmux show-options -t {0} -pv @server_pass); [ -n \"$PASS\" ] && tmux send-keys -t {0} \"$PASS\" Enter", current_pane_id);
    Command::new("bash").arg("-c").arg(cmd).spawn()?;

    println!("Connecting to {}...", host);
    let _ = Command::new("ssh").arg(format!("{}@{}", selection.user, host)).exec();
    
    Ok(())
}

// --- Helpers ---

fn get_rbw_items(folder: &str) -> Result<Vec<RbwItem>> {
    ensure_rbw_ready()?;
    let output = Command::new("rbw").args(["list", "--raw"]).output()?;
    let items: Vec<RbwItem> = serde_json::from_slice(&output.stdout)?;
    Ok(items.into_iter().filter(|i| i.folder.as_deref() == Some(folder)).collect())
}

fn get_server_details(items: Vec<RbwItem>) -> Vec<Server> {
    items.into_par_iter().filter_map(|item| {
        let out = Command::new("rbw").args(["get", &item.id, "--raw"]).output().ok()?;
        let full: RbwItemFull = serde_json::from_slice(&out.stdout).ok()?;
        let d = full.data?;
        Some(Server {
            id: full.id,
            name: full.name,
            group: full.organization.unwrap_or_else(|| "Personal".into()),
            user: d.username.unwrap_or_default(),
            pass: d.password.unwrap_or_default(),
            uris: d.uris.unwrap_or_default().into_iter().map(|u| u.uri).collect(),
        })
    }).collect()
}

fn get_snippet_details(items: Vec<RbwItem>) -> Vec<Snippet> {
    items.into_par_iter().filter_map(|item| {
        let out = Command::new("rbw").args(["get", &item.id, "--raw"]).output().ok()?;
        let full: RbwItemFull = serde_json::from_slice(&out.stdout).ok()?;
        let notes = full.notes?;
        Some(Snippet {
            name: full.name,
            group: full.organization.unwrap_or_else(|| "Personal".into()),
            notes,
        })
    }).collect()
}

fn sort_by_group_and_name<T, FG, FN>(items: &mut [T], group_fn: FG, name_fn: FN) 
where FG: Fn(&T) -> &str, FN: Fn(&T) -> &str 
{
    items.sort_by(|a, b| {
        let ga = group_fn(a);
        let gb = group_fn(b);
        if ga == gb { name_fn(a).cmp(name_fn(b)) }
        else if ga == "Personal" { std::cmp::Ordering::Less }
        else if gb == "Personal" { std::cmp::Ordering::Greater }
        else { ga.cmp(gb) }
    });
}

// --- Main Scalabil ---

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let result = if args.iter().any(|a| a == "--snippets") {
        run_snippets()
    } else {
        run_servers()
    };

    if let Err(e) = result {
        // Dacă eroarea este "CANCELLED", ieșim discret
        if e.to_string() == "CANCELLED" {
            std::process::exit(0);
        }
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}