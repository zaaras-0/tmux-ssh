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

        let email = "adq0p0@gmail.com".to_string();
        let server_url = "https://vault.znest.ro".to_string();

        let final_server_url = if server_url == "https://vault.bitwarden.com" {
            String::new() // SDK defaults to official Cloud
        } else {
            server_url
        };

        // Creăm o configurație temporară pentru login
        let mut config = Config {
            email,
            server_url: final_server_url,
            personal_folder: String::new(),
            organizations: Vec::new(),
        };

        // 2. Login
        let client = auth::login_wizard(&config).await?;

        // 3. Extragere foldere folosind sesiunea proaspătă
        println!("🔍 Se încarcă folderele din Vault...");
        let folders = vault::list_folders(&client).await?; 
        
        if folders.is_empty() {
            return Err(anyhow::anyhow!("Nu s-au găsit foldere în acest cont. Creați unul în Bitwarden mai întâi."));
        }

        let folder_names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        let personal_folder = prompts::select_from_list("Alege folderul Personal pentru SSH/Snippets", folder_names)?;

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
                    let selected_coll = prompts::select_from_list(&format!("Alege colecția din '{}'", org.name), coll_names)?;

                    selected_orgs.push(OrgConfig {
                        name: org.name,
                        collections: vec![selected_coll],
                    });
                }
            }
        }

        config.personal_folder = personal_folder;
        config.organizations = selected_orgs;

        config.save()?;
        println!("✅ Configurare salvată cu succes!");
        Ok(config)
    }
}
