use std::process::Command;
use anyhow::{Result, anyhow, Context};
use crate::models::*;

use bitwarden::{
    vault::item::ItemResponse,
    BitwardenClient,
};

/// Helper generic pentru a rula comenzi 'bw list' și a parsa JSON-ul
fn bw_list<T: serde::de::DeserializeOwned>(category: &str, session: &str, extra_args: Vec<&str>) -> Result<Vec<T>> {
    let mut cmd = Command::new("bw");
    cmd.args(["list", category, "--session", session]);
    
    for arg in extra_args {
        cmd.arg(arg);
    }

    let output = cmd.output().context(format!("Eșec la executarea 'bw list {}'", category))?;
    
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Eroare BW CLI: {}", err));
    }

    let data: Vec<T> = serde_json::from_slice(&output.stdout)
        .context(format!("Eșec la parsarea JSON pentru {}", category))?;
    
    Ok(data)
}

/// Listează toate folderele personale (pentru setup)
pub fn list_folders(session: &str) -> Result<Vec<BwFolder>> {
    bw_list("folders", session, vec![])
}

/// Listează toate organizațiile din care face parte utilizatorul
pub fn list_organizations(session: &str) -> Result<Vec<BwOrganization>> {
    bw_list("organizations", session, vec![])
}

/// Listează colecțiile dintr-o organizație specifică
pub fn list_collections(session: &str, org_id: &str) -> Result<Vec<BwCollection>> {
    bw_list("collections", session, vec!["--organizationid", org_id])
}

/// Extrage toate item-urile filtrate conform configurației (Personal + Orgs)
pub async fn fetch_filtered_items(config: &Config, client: &mut BitwardenClient) -> Result<Vec<ItemResponse>> {
    // 1. Luăm toate itemele din seif prin SDK
    let items_output = client.vault().list(None).await
        .map_err(|e| anyhow::anyhow!("Eroare SDK listare: {:?}", e))?;

    // 2. Filtrăm itemele (doar Logins și doar din folderul setat în Config)
    let filtered: Vec<ItemResponse> = items_output.data.into_iter()
        .filter(|item| {
            // Verificăm dacă este un Login (Type 1)
            let is_login = item.item_type == bitwarden::vault::item::ItemType::Login;
            
            // Verificăm folderul (comparăm numele folderului din config cu cel din seif)
            // Notă: SDK-ul returnează folder_id, deci s-ar putea să avem nevoie 
            // de o listă de foldere cache-uită pentru a compara numele.
            is_login
        })
        .collect();

    Ok(filtered)
}