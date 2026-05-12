// Copyright (C) 2026 [Numele Tău]
// Licensed under GNU GPL v3

mod models;
mod config;
mod auth;
mod vault;
mod prompts;
mod ssh;
mod snippets;
mod sftp;

use anyhow::{Context, Result};
use crate::models::Config;
use skim::prelude::*;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    // Verificăm flag-ul de refresh (-r sau --refresh)
    let force_refresh = args.iter().any(|a| a == "-r" || a == "--refresh") || command == "sync";

    // 1. Verificăm Status (doar dacă vault-ul e deblocat - sesiunea e validă)
    if command == "status" {
        if auth::get_active_session().is_ok() {
            println!("🔓 Vault unlocked (Session active)");
        } else {
            println!("🔒 Vault locked");
        }
        return Ok(());
    }

    // 2. Comenzi de mentenanță (nu necesită neapărat config valid)
    match command {
        "lock" => {
            return auth::purge_session(); // Lock doar șterge sesiunea din RAM (/dev/shm)
        },
        "purge" => {
            auth::purge_session()?; // Ștergem sesiunea
            let path = Config::get_path()?;
            if path.exists() {
                std::fs::remove_file(path)?;
                println!("🗑️ Configurația a fost ștearsă.");
            }
            return Ok(());
        },
        _ => {}
    }

    // 3. Verificare Config & Auto-Wizard
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(_) => {
            Config::run_wizard().await?
        }
    };

    // 4. Dispatcher Comenzi Principale
    match command {
        "sync" => {
            let mut client = auth::get_client(&config).await?;
            println!("🔄 Se sincronizează datele din Vault...");
            vault::fetch_filtered_items(&config, &mut client, false, true).await?;
            println!("✅ Sincronizare finalizată și cache actualizat.");
        },
        "config" => {
            Config::run_wizard().await?;
        },
        "login" | "unlock" => {
            auth::login_wizard(&config).await?;
        },
        "list" | "ssh" => {
            run_list_flow(&config, false, None, force_refresh).await?;
        },
        "search" => {
            // Căutăm query-ul excluzând flag-urile și comanda search
            let query = args.iter()
                .filter(|a| !a.starts_with('-') && *a != "search" && *a != command && !env::current_exe().unwrap().to_string_lossy().contains(*a))
                .nth(0)
                .cloned();
                
            let final_query = match query {
                Some(q) => Some(q),
                None => Some(prompts::ask_input("Caută în Vault", None)?),
            };
            run_list_flow(&config, false, final_query, force_refresh).await?;
        },
        "snip" | "snippets" => {
            run_list_flow(&config, true, None, force_refresh).await?;
        },
        "pass" => {
            ssh::inject_password_from_tmux()?;
        },
        "sftp" => {
            sftp::run_sftp_flow(&config).await?;
        },
        "_connect" => {
            let id = args.get(2).context("Lipsă ID item")?;
            let ip = args.get(3).cloned();
            ssh::execute_ssh_internal(&config, id, ip).await?;
        },
        _ => {
            println!("Comandă necunoscută: {}. Utilizați: list, search, ssh, snippets, sync, config, login, lock, purge, status.", command);
        }
    }

    Ok(())
}

async fn run_list_flow(config: &Config, is_snippet: bool, query: Option<String>, force_refresh: bool) -> Result<()> {
    let mut client = auth::get_client(config).await?;
    
    println!("🔍 Se încarcă datele...");
    let mut items = vault::fetch_filtered_items(config, &mut client, is_snippet, force_refresh).await?;
    
    // Sort items alphabetically by name
    items.sort_by(|a, b| {
        let name_a = a.name.as_deref().unwrap_or("");
        let name_b = b.name.as_deref().unwrap_or("");
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });

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
        let name = item.name.as_deref().unwrap_or("Unknown");
        input_data.push_str(&format!("{} {}\n", prefix, name));
    }

    let mut builder = SkimOptionsBuilder::default();
    builder
        .height(Some("100%"))
        .reverse(true)
        .margin(Some("1,1,1,1"))
        .multi(false)
        .header(Some(if is_snippet { "Selectează Snippet" } else { "Selectează Server SSH" }))
        .prompt(Some("🔎 > "));
    
    let query_str = query.clone().unwrap_or_default();
    if query.is_some() {
        builder.query(Some(&query_str));
    }

    let options = builder.build().unwrap();

    let item_reader = SkimItemReader::default();
    let items_stream = item_reader.of_bufread(std::io::Cursor::new(input_data));

    if let Some(out) = Skim::run_with(&options, Some(items_stream)) {
        if out.is_abort {
            return Ok(());
        }

        let selected_item = out.selected_items.get(0)
            .context("Nu s-a selectat niciun item")?;
        
        let selected_output = selected_item.output();

        let chosen_item = items.into_iter()
            .find(|item| {
                let prefix = if item.organization_id.is_some() { "[Org]" } else { "[Personal]" };
                let name = item.name.as_deref().unwrap_or("Unknown");
                format!("{} {}", prefix, name) == selected_output
            })
            .context("Eroare la recuperarea item-ului din listă")?;

        if is_snippet {
            snippets::execute_snippet(&client, chosen_item)?;
        } else {
            // Verificăm dacă avem mai multe URI-uri
            let uris = chosen_item.login.as_ref()
                .and_then(|l| l.uris.as_ref())
                .cloned()
                .unwrap_or_default();
            
            let mut decrypted_uris = Vec::new();
            for u in uris {
                if let Some(enc_uri) = u.uri {
                    if let Ok(dec_uri) = vault::decrypt_string(&client, &enc_uri, chosen_item.organization_id.as_deref()) {
                        decrypted_uris.push(dec_uri);
                    }
                }
            }

            let selected_ip = if decrypted_uris.len() > 1 {
                // Afișăm un sub-selector pentru IP
                let mut uri_input = String::new();
                for u in &decrypted_uris {
                    uri_input.push_str(&format!("{}\n", u));
                }

                let mut sub_builder = SkimOptionsBuilder::default();
                sub_builder
                    .height(Some("100%"))
                    .reverse(true)
                    .margin(Some("1,2,1,2"))
                    .header(Some("Selectează IP/Host"))
                    .prompt(Some("🌐 > "));
                
                let sub_options = sub_builder.build().unwrap();
                let sub_reader = SkimItemReader::default();
                let sub_stream = sub_reader.of_bufread(std::io::Cursor::new(uri_input));

                if let Some(sub_out) = Skim::run_with(&sub_options, Some(sub_stream)) {
                    if sub_out.is_abort {
                        return Ok(());
                    }
                    Some(sub_out.selected_items.get(0)
                        .map(|i| i.output().to_string())
                        .context("Nu s-a selectat niciun URI")?)
                } else {
                    return Ok(());
                }
            } else {
                decrypted_uris.first().cloned()
            };

            ssh::spawn_ssh_session(&chosen_item, selected_ip)?;
        }
    }

    Ok(())
}
