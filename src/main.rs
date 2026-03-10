mod models;
mod config;
mod auth;
mod vault;
mod prompts;
mod ssh;
mod snippets;

use anyhow::{Context, Result};
use crate::models::Config;
use skim::prelude::*;
use std::io::Cursor;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    // 1. Verificare Config & Auto-Wizard
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(_) => {
            // Permitem purge chiar dacă nu avem config
            if command == "purge" { 
                auth::purge_session()?;
                return Ok(());
            }
            // Altfel, forțăm configurarea
            Config::run_wizard()?
        }
    };

    // 2. Dispatcher Comenzi
    match command {
        "config" => {
            Config::run_wizard()?;
        },
        "login" | "unlock" => {
            auth::login_wizard(&config.email)?;
        },
        "lock" | "purge" => {
            auth::purge_session()?;
        },
        "list" | "search" | "ssh" => {
            run_list_flow(&config, false)?;
        },
        "snip" | "snippets" => {
            run_list_flow(&config, true)?;
        },
        "status" => {
            let session = auth::get_active_session().ok();
            let status = auth::check_status(&session)?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        },
        _ => {
            println!("Comandă necunoscută: {}. Utilizați: list, ssh, snippets, config, login, lock, purge.", command);
        }
    }

    Ok(())
}

fn run_list_flow(config: &Config, is_snippet: bool) -> Result<()> {
    let session = auth::get_active_session()?;
    
    println!("🔍 Se încarcă datele din Vault...");
    let items = vault::fetch_filtered_items(config, &session)?;
    
    if items.is_empty() {
        println!("⚠️ Nu s-au găsit iteme în locațiile configurate.");
        return Ok(());
    }

    let mut input_data = String::new();
    for item in &items {
        let prefix = if item.organization_id.is_some() {
            "[Org]"
        } else {
            "[Personal]"
        };
        input_data.push_str(&format!("{} {}\n", prefix, item.name));
    }

    let options = SkimOptionsBuilder::default()
        .height(Some("40%"))
        .multi(false)
        .build()
        .unwrap();

    let item_reader = SkimItemReader::default();
    let items_stream = item_reader.of_bufread(Cursor::new(input_data));

    if let Some(out) = Skim::run_with(&options, Some(items_stream)) {
        if out.is_abort {
            return Ok(());
        }

        let selected_item = out.selected_items.get(0)
            .context("Nu s-a selectat niciun item")?;
        
        // Luăm textul afișat în skim
        let selected_output = selected_item.output();

        // Găsim item-ul în vectorul original care corespunde cu ce am formatat la pasul 3
        let chosen_item = items.into_iter()
            .find(|item| {
                let prefix = if item.organization_id.is_some() { "[Org]" } else { "[Personal]" };
                format!("{} {}", prefix, item.name) == selected_output
            })
            .context("Eroare la recuperarea item-ului din listă")?;

        if is_snippet {
            snippets::execute_snippet(chosen_item)?;
        } else {
            ssh::execute_ssh(chosen_item)?;
        }
    }

    Ok(())
}