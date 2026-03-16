use crate::models::{Config, OrgConfig};
use crate::prompts;
use crate::auth;
use crate::vault;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::PathBuf;

impl Config {
    /// Returnează calea către ~/.config/zbw/config.json
    pub fn get_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir().context("Nu s-a putut găsi directorul de config")?;
        path.push("zbw");
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        path.push("config.json");
        Ok(path)
    }

    /// Încarcă configurația de pe disk
    pub fn load() -> Result<Self> {
        let path = Self::get_path()?;
        if !path.exists() {
            return Err(anyhow!("Configurația lipsește. Rulați 'zbw config'."));
        }
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Salvează configurația pe disk
    pub fn save(&self) -> Result<()> {
        let path = Self::get_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Wizard-ul interactiv de configurare
    pub async fn run_wizard() -> Result<Self> {
        println!("🚀 Începem configurarea zbw...");
        
        let existing_config = Self::load().ok();

        // 1. Identitate (Folosim valorile din config dacă există)
        let default_email = existing_config.as_ref().map(|c| c.email.as_str()).unwrap_or("adq0p0@gmail.com");
        let default_server = existing_config.as_ref().map(|c| c.server_url.as_str()).unwrap_or("https://vault.znest.ro");

        let email = prompts::ask_input("Email Bitwarden", Some(default_email.to_string()))?;
        let server_url = prompts::ask_input("Server URL (Vaultwarden)", Some(default_server.to_string()))?;

        let final_server_url = if server_url == "https://vault.bitwarden.com" {
            String::new() 
        } else {
            server_url.clone()
        };

        let mut config = Config {
            email: email.clone(),
            server_url: final_server_url,
            personal_folder: String::new(),
            personal_snippets_folder: String::new(),
            organizations: Vec::new(),
        };

        // 2. Login
        let client = auth::login_wizard(&config).await?;

        // 3. Extragere foldere pentru Personal (Servers & Snippets)
        println!("🔍 Se încarcă folderele din Vault...");
        let folders = vault::list_folders(&client).await?; 
        
        if folders.is_empty() {
            return Err(anyhow::anyhow!("Nu s-au găsit foldere în acest cont. Creați unul în Bitwarden mai întâi."));
        }

        let folder_names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        
        // Defaults pentru foldere
        let default_srv_folder = existing_config.as_ref()
            .map(|c| c.personal_folder.as_str())
            .unwrap_or_else(|| {
                if folder_names.contains(&"Servers") { "Servers" } else { folder_names[0] }
            });
            
        let default_snip_folder = existing_config.as_ref()
            .map(|c| c.personal_snippets_folder.as_str())
            .unwrap_or_else(|| {
                if folder_names.contains(&"Snippets") { "Snippets" } else { folder_names[0] }
            });

        let personal_folder = prompts::select_from_list_with_default("Alege folderul Personal pentru SERVERE", folder_names.clone(), default_srv_folder)?;
        let personal_snippets_folder = prompts::select_from_list_with_default("Alege folderul Personal pentru SNIPPETS", folder_names, default_snip_folder)?;

        // 4. Configurare Organizații (Opțional)
        let mut selected_orgs = Vec::new();
        if prompts::ask_confirm("Doriți să adăugați și iteme dintr-o Organizație?")? {
            let orgs = vault::list_organizations(&client).await?;
            for org in orgs {
                if prompts::ask_confirm(&format!("Includeți organizația '{}'?", org.name))? {
                    let collections = vault::list_collections(&client, &org.id).await?;
                    if collections.is_empty() {
                        println!("⚠️ Nu s-au găsit colecții în organizația '{}'.", org.name);
                        continue;
                    }

                    let coll_names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
                    let selected_coll = prompts::select_from_list("Alege colecția pentru SERVERE", coll_names.clone())?;
                    let selected_snip_coll = prompts::select_from_list("Alege colecția pentru SNIPPETS", coll_names)?;

                    selected_orgs.push(OrgConfig {
                        name: org.name,
                        collections: vec![selected_coll],
                        snippets_collections: vec![selected_snip_coll],
                    });
                }
            }
        }

        config.personal_folder = personal_folder;
        config.personal_snippets_folder = personal_snippets_folder;
        config.organizations = selected_orgs;

        config.save()?;
        println!("✅ Configurare salvată cu succes!");
        Ok(config)
    }
}
