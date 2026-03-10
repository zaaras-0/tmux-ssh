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
            return Err(anyhow!("Configurația lipsește. Rulați 'zbw config' sau 'zbw login'."));
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
    pub fn run_wizard() -> Result<Self> {
        println!("🚀 Începem configurarea zbw...");

        let email = prompts::ask_input("Email Bitwarden", None)?;
        let server_url = prompts::ask_input("Server URL", Some("https://vault.bitwarden.com".to_string()))?;

        // 1. Setăm serverul (acum forțează logout dacă e nevoie)
        auth::set_bw_server(&server_url)?;

        // 2. Login - Acesta va returna sesiunea necesară pentru pașii următori
        // Aici utilizatorul va introduce parola și 2FA în terminal
        let session = auth::login_wizard(&email)?;

        // 3. Extragere foldere folosind sesiunea proaspătă
        println!("🔍 Se încarcă folderele din Vault...");
        let folders = vault::list_folders(&session)?; 
        
        if folders.is_empty() {
            return Err(anyhow::anyhow!("Nu s-au găsit foldere în acest cont. Creați unul în Bitwarden mai întâi."));
        }

        let folder_names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        let personal_folder = prompts::select_from_list("Alege folderul Personal pentru SSH/Snippets", folder_names)?;

        // 4. Configurare Organizații (Opțional)
        let mut selected_orgs = Vec::new();
        if prompts::ask_confirm("Doriți să adăugați și iteme dintr-o Organizație?")? {
            let orgs = vault::list_organizations(&session)?;
            for org in orgs {
                if prompts::ask_confirm(&format!("Includeți organizația '{}'?", org.name))? {
                    // Aici am putea lista și colecțiile, dar pentru MVP includem tot din Org
                    selected_orgs.push(OrgConfig {
                        name: org.name,
                        collections: Vec::new(), // Gol înseamnă "toate"
                    });
                }
            }
        }

        let config = Config {
            email,
            server_url,
            personal_folder,
            organizations: selected_orgs,
        };

        config.save()?;
        println!("✅ Configurare salvată cu succes!");
        Ok(config)
    }
}