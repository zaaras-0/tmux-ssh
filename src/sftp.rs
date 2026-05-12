use std::net::TcpStream;
use std::path::PathBuf;
use anyhow::{Result, Context, anyhow};
use ssh2::Session;
use crate::models::Config;
use crate::ssh::get_server_details;
use crate::auth;
use crate::vault;
use skim::prelude::*;

pub async fn run_sftp_flow(config: &Config) -> Result<()> {
    let mut client = auth::get_client(config).await?;
    
    // 1. Select Direction
    let directions = vec!["📤 Upload (Local -> Server)", "📥 Download (Server -> Local)", "🔄 Relay (Server -> Server)"];
    let direction = crate::prompts::select_from_list("Alege direcția transferului", directions)?;

    match direction.as_str() {
        d if d.contains("Upload") => upload_flow(config, &mut client).await?,
        d if d.contains("Download") => download_flow(config, &mut client).await?,
        d if d.contains("Relay") => relay_flow(config, &mut client).await?,
        _ => return Err(anyhow!("Direcție invalidă")),
    }

    Ok(())
}

async fn upload_flow(config: &Config, client: &mut bitwarden_core::Client) -> Result<()> {
    // 1. Pick Local File
    let local_file = local_browser(None, false).context("Nu s-a selectat niciun fișier local")?;
    
    // 2. Select Server
    let server = pick_server(config, client).await?;
    let (_session, sftp) = establish_sftp_session(config, client, &server.id).await?;
    
    // 3. Pick Remote Destination Dir
    let mut remote_path = remote_browser(&sftp, ".", true).context("Nu s-a selectat niciun director la distanță")?;
    
    // Dacă am selectat un director, adăugăm numele fișierului sursă
    if sftp.stat(&remote_path).map(|s| s.is_dir()).unwrap_or(false) {
        if let Some(file_name) = local_file.file_name() {
            remote_path.push(file_name);
        }
    }

    let srv_name = server.name.as_deref().unwrap_or("Unknown");
    println!("📤 Se încarcă {} în {}:{}...", local_file.display(), srv_name, remote_path.display());
    
    let mut remote_file = sftp.create(&remote_path).context("Nu s-a putut crea fișierul la distanță")?;
    let mut local_file_handle = std::fs::File::open(&local_file).context("Nu s-a putut deschide fișierul local")?;
    
    std::io::copy(&mut local_file_handle, &mut remote_file).context("Eroare la transferul datelor (upload)")?;

    println!("✅ Upload finalizat cu succes.");
    Ok(())
}

async fn download_flow(config: &Config, client: &mut bitwarden_core::Client) -> Result<()> {
    // 1. Select Server
    let server = pick_server(config, client).await?;
    let (_session, sftp) = establish_sftp_session(config, client, &server.id).await?;

    // 2. Pick Remote File
    let remote_file_path = remote_browser(&sftp, ".", false).context("Nu s-a selectat niciun fișier la distanță")?;

    // 3. Pick Local Destination Dir
    let mut local_path = local_browser(None, true).context("Nu s-a selectat niciun director local")?;
    
    // Dacă am selectat un director, adăugăm numele fișierului sursă
    if local_path.is_dir() {
        if let Some(file_name) = remote_file_path.file_name() {
            local_path.push(file_name);
        }
    }

    let srv_name = server.name.as_deref().unwrap_or("Unknown");
    println!("📥 Se descarcă {}:{} în {}...", srv_name, remote_file_path.display(), local_path.display());

    let mut remote_file_handle = sftp.open(&remote_file_path).context("Nu s-a putut deschide fișierul la distanță")?;
    let mut local_file_handle = std::fs::File::create(&local_path).context("Nu s-a putut crea fișierul local")?;

    std::io::copy(&mut remote_file_handle, &mut local_file_handle).context("Eroare la transferul datelor (download)")?;

    println!("✅ Download finalizat cu succes.");
    Ok(())
}

fn remote_browser(sftp: &ssh2::Sftp, start_path: &str, dirs_only: bool) -> Option<PathBuf> {
    let mut current_path = PathBuf::from(start_path);
    if current_path.as_os_str().is_empty() { current_path.push("."); }

    loop {
        let entries = match sftp.readdir(&current_path) {
            Ok(e) => e,
            Err(err) => {
                println!("❌ Eroare la citirea directorului: {}", err);
                return None;
            }
        };

        let mut input_data = String::new();
        // Option to select current directory if in dirs_only mode
        if dirs_only {
            input_data.push_str(". [Selectează acest director]\n");
        }
        input_data.push_str(".. [Înapoi]\n");

        for (path, stat) in &entries {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let suffix = if stat.is_dir() { "/" } else { "" };
            
            if dirs_only && !stat.is_dir() { continue; }
            
            input_data.push_str(&format!("{}{}\n", name, suffix));
        }

        let mut builder = SkimOptionsBuilder::default();
        let header = format!("Remote: {}", current_path.display());
        builder
            .height(Some("100%"))
            .reverse(true)
            .header(Some(&header))
            .prompt(Some("📁 > "));
        
        let options = builder.build().unwrap();
        let item_reader = SkimItemReader::default();
        let items_stream = item_reader.of_bufread(std::io::Cursor::new(input_data));

        if let Some(out) = Skim::run_with(&options, Some(items_stream)) {
            if out.is_abort { return None; }

            let selected = out.selected_items.get(0)?.output();
            if selected == ".. [Înapoi]" {
                current_path.pop();
            } else if selected == ". [Selectează acest director]" {
                return Some(current_path);
            } else {
                let clean_name = selected.trim_end_matches('/');
                let mut next_path = current_path.clone();
                next_path.push(clean_name);

                // Verificăm dacă e director
                let is_dir = entries.iter().find(|(p, _)| p.file_name().unwrap_or_default().to_string_lossy() == clean_name)
                    .map(|(_, s)| s.is_dir()).unwrap_or(false);

                if is_dir {
                    current_path = next_path;
                } else {
                    if dirs_only {
                        println!("⚠️ Te rugăm să selectezi un director.");
                    } else {
                        return Some(next_path);
                    }
                }
            }
        } else {
            return None;
        }
    }
}

fn local_browser(start_path: Option<PathBuf>, dirs_only: bool) -> Option<PathBuf> {
    let mut current_path = start_path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    loop {
        let entries = match std::fs::read_dir(&current_path) {
            Ok(e) => e,
            Err(err) => {
                println!("❌ Eroare la citirea directorului local: {}", err);
                return None;
            }
        };

        let mut input_data = String::new();
        if dirs_only {
            input_data.push_str(". [Selectează acest director]\n");
        }
        input_data.push_str(".. [Înapoi]\n");

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let suffix = if is_dir { "/" } else { "" };
            
            if dirs_only && !is_dir { continue; }
            
            input_data.push_str(&format!("{}{}\n", name, suffix));
        }

        let mut builder = SkimOptionsBuilder::default();
        let header = format!("Local: {}", current_path.display());
        builder
            .height(Some("100%"))
            .reverse(true)
            .header(Some(&header))
            .prompt(Some("📁 > "));
        
        let options = builder.build().unwrap();
        let item_reader = SkimItemReader::default();
        let items_stream = item_reader.of_bufread(std::io::Cursor::new(input_data));

        if let Some(out) = Skim::run_with(&options, Some(items_stream)) {
            if out.is_abort { return None; }

            let selected = out.selected_items.get(0)?.output();
            if selected == ".. [Înapoi]" {
                current_path.pop();
            } else if selected == ". [Selectează acest director]" {
                return Some(current_path);
            } else {
                let clean_name = selected.trim_end_matches('/');
                let mut next_path = current_path.clone();
                next_path.push(clean_name);

                if next_path.is_dir() {
                    current_path = next_path;
                } else {
                    if dirs_only {
                        println!("⚠️ Te rugăm să selectezi un director.");
                    } else {
                        return Some(next_path);
                    }
                }
            }
        } else {
            return None;
        }
    }
}

async fn relay_flow(config: &Config, client: &mut bitwarden_core::Client) -> Result<()> {
    // 1. Server Sursă
    println!("--- Selectează Server Sursă ---");
    let server_a = pick_server(config, client).await?;
    let (_sess_a, sftp_a) = establish_sftp_session(config, client, &server_a.id).await?;
    let remote_file_a = remote_browser(&sftp_a, ".", false).context("Nu s-a selectat fișierul sursă")?;

    // 2. Server Destinație
    println!("--- Selectează Server Destinație ---");
    let server_b = pick_server(config, client).await?;
    let (_sess_b, sftp_b) = establish_sftp_session(config, client, &server_b.id).await?;
    let mut remote_path_b = remote_browser(&sftp_b, ".", true).context("Nu s-a selectat directorul destinație")?;

    if sftp_b.stat(&remote_path_b).map(|s| s.is_dir()).unwrap_or(false) {
        if let Some(file_name) = remote_file_a.file_name() {
            remote_path_b.push(file_name);
        }
    }

    let srv_a_name = server_a.name.as_deref().unwrap_or("Source");
    let srv_b_name = server_b.name.as_deref().unwrap_or("Dest");
    println!("🔄 Se transferă {} -> {}...", srv_a_name, srv_b_name);

    let temp_path = std::env::temp_dir().join(format!("zbw_relay_{}", uuid::Uuid::new_v4()));
    
    {
        let mut remote_file_a_handle = sftp_a.open(&remote_file_a).context("Nu s-a putut deschide fișierul sursă")?;
        let mut temp_file = std::fs::File::create(&temp_path).context("Nu s-a putut crea fișierul temporar")?;
        std::io::copy(&mut remote_file_a_handle, &mut temp_file).context("Eroare la descărcarea în temp")?;
    }

    {
        let mut temp_file = std::fs::File::open(&temp_path).context("Nu s-a putut redeschide fișierul temporar")?;
        let mut remote_file_b_handle = sftp_b.create(&remote_path_b).context("Nu s-a putut crea fișierul destinație")?;
        std::io::copy(&mut temp_file, &mut remote_file_b_handle).context("Eroare la încărcarea din temp")?;
    }

    let _ = std::fs::remove_file(&temp_path);

    println!("✅ Relay finalizat cu succes.");
    Ok(())
}

async fn pick_server(config: &Config, client: &mut bitwarden_core::Client) -> Result<crate::models::BwCipher> {
    let items = vault::fetch_filtered_items(config, client, false, false).await?;
    
    let mut input_data = String::new();
    for item in &items {
        let prefix = if item.organization_id.is_some() { "[Org]" } else { "[Personal]" };
        let name = item.name.as_deref().unwrap_or("Unknown");
        input_data.push_str(&format!("{} {}\n", prefix, name));
    }

    let mut builder = SkimOptionsBuilder::default();
    builder
        .height(Some("100%"))
        .reverse(true)
        .header(Some("Selectează Server"))
        .prompt(Some("🔎 > "));
    
    let options = builder.build().unwrap();
    let item_reader = SkimItemReader::default();
    let items_stream = item_reader.of_bufread(std::io::Cursor::new(input_data));

    if let Some(out) = Skim::run_with(&options, Some(items_stream)) {
        if out.is_abort {
            return Err(anyhow!("Selecție anulată"));
        }

        let selected_item = out.selected_items.get(0).context("Nu s-a selectat niciun server")?;
        let selected_output = selected_item.output();

        let chosen_item = items.into_iter()
            .find(|item| {
                let prefix = if item.organization_id.is_some() { "[Org]" } else { "[Personal]" };
                let name = item.name.as_deref().unwrap_or("Unknown");
                format!("{} {}", prefix, name) == selected_output
            })
            .context("Eroare la recuperarea item-ului")?;
        
        Ok(chosen_item)
    } else {
        Err(anyhow!("Eroare la rularea selectorului"))
    }
}

async fn establish_sftp_session(
    config: &Config, 
    client: &mut bitwarden_core::Client, 
    server_id: &str
) -> Result<(Session, ssh2::Sftp)> {
    // În viitor am putea lăsa user-ul să aleagă IP-ul dacă sunt mai multe. 
    // Deocamdată luăm default (None pentru selected_ip).
    let details = get_server_details(config, client, server_id, None).await?;
    
    let tcp = TcpStream::connect(format!("{}:{}", details.host, details.port))
        .context(format!("Nu s-a putut conecta la {}:{}", details.host, details.port))?;
    
    let mut sess = Session::new().context("Nu s-a putut crea sesiunea SSH")?;
    sess.set_tcp_stream(tcp);
    sess.handshake().context("SSH handshake eșuat")?;
    
    if let Some(password) = details.password {
        sess.userauth_password(&details.username, &password)
            .context("Autentificare SSH eșuată")?;
    } else {
        // Încercăm agent-ul dacă nu avem parolă? Deocamdată Bitwarden ar trebui să aibă parole.
        sess.userauth_agent(&details.username).context("Autentificare prin agent eșuată")?;
    }

    let sftp = sess.sftp().context("Nu s-a putut inițializa SFTP")?;
    
    Ok((sess, sftp))
}
