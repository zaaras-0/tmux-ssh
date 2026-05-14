use crate::models::Config;
use crate::ssh::{pick_server, establish_ssh_session, execute_ssh_internal};
use crate::auth;
use crate::prompts;
use anyhow::{Result, Context, anyhow};
use skim::prelude::*;
use std::io::Read;

pub async fn run_docker_flow(config: &Config) -> Result<()> {
    let mut client = auth::get_client(config).await?;
    
    // 1. Select Server
    let server = pick_server(config, &mut client).await?;
    
    println!("🔍 Se conectează la {} și se preia lista de containere...", server.name.as_deref().unwrap_or("Server"));
    
    // 2. Establish Session and Fetch Containers
    let sess = establish_ssh_session(config, &mut client, &server.id).await?;
    let mut channel = sess.channel_session().context("Nu s-a putut deschide canalul SSH")?;
    
    // Command to list containers with specific format for parsing
    channel.exec("docker ps --format '{{.ID}}|{{.Names}}|{{.Status}}|{{.Image}}'").context("Execuția docker ps eșuată")?;
    
    let mut output = String::new();
    channel.read_to_string(&mut output).context("Nu s-a putut citi lista de containere")?;
    
    if output.trim().is_empty() {
        return Err(anyhow!("Nu s-au găsit containere active sau Docker nu este instalat pe acest server."));
    }

    // 3. Select Container via Skim
    let mut skim_input = String::new();
    let lines: Vec<&str> = output.lines().collect();
    for line in &lines {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            // Format: [Name] (Image) - Status
            skim_input.push_str(&format!("[{}] ({}) - {}\n", parts[1], parts[3], parts[2]));
        }
    }

    let mut builder = SkimOptionsBuilder::default();
    builder
        .height(Some("100%"))
        .reverse(true)
        .header(Some("Selectează Container Docker"))
        .prompt(Some("🐳 > "));
    
    let options = builder.build().unwrap();
    let item_reader = SkimItemReader::default();
    let items_stream = item_reader.of_bufread(std::io::Cursor::new(skim_input));

    if let Some(out) = Skim::run_with(&options, Some(items_stream)) {
        if out.is_abort { return Ok(()); }

        let selected = out.selected_items.get(0).context("Nu s-a selectat niciun container")?.output();
        
        // Match back to ID
        let container_id = lines.iter().find(|l| {
            let p: Vec<&str> = l.split('|').collect();
            p.len() >= 2 && format!("[{}]", p[1]) == selected.split(" (").next().unwrap_or("")
        })
        .map(|l| l.split('|').next().unwrap_or(""))
        .context("Nu s-a putut identifica ID-ul containerului")?;

        // 4. Select Action
        let actions = vec!["💻 Exec (Interactive Shell)", "📄 Logs (Follow)"];
        let action = prompts::select_from_list("Alege acțiunea", actions)?;

        let remote_cmd = if action.contains("Exec") {
            format!("docker exec -it {} sh -c '[ -x /bin/bash ] && exec /bin/bash || exec /bin/sh'", container_id)
        } else {
            format!("docker logs -f {}", container_id)
        };

        // 5. Execute Interactive SSH
        execute_ssh_internal(config, &server.id, None, Some(&remote_cmd)).await?;
    }

    Ok(())
}
