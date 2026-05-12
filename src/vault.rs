use anyhow::{Result, anyhow, Context};
use bitwarden_core::Client;
use bitwarden_crypto::{KeyDecryptable, KeyEncryptable, EncString};
use serde_json::Value;
use std::str::FromStr;
use crate::models::*;

use std::fs;
use std::path::PathBuf;

/// Extrage toate item-urile filtrate conform configurației (Personal + Orgs)
pub async fn fetch_filtered_items(config: &Config, client: &mut Client, is_snippet: bool, force_refresh: bool) -> Result<Vec<BwCipher>> {
    let sync_data = fetch_sync_with_reauth(client, config, force_refresh).await?;

    let ciphers_val = sync_data["ciphers"].as_array().context("Nu am găsit ciphers în sync")?;
    let mut ciphers: Vec<BwCipher> = serde_json::from_value(serde_json::Value::Array(ciphers_val.clone()))?;

    // 1. Decriptăm folderele pentru a găsi ID-ul folderului personal ales
    let folders = list_folders_from_sync(&sync_data, client).await?;
    
    let target_folder_name = if is_snippet {
        &config.personal_snippets_folder
    } else {
        &config.personal_folder
    };

    let personal_folder_id = folders.iter()
        .find(|f| &f.name == target_folder_name)
        .and_then(|f| f.id.clone());

    // 2. Extragem ID-urile colecțiilor selectate din organizații
    let mut selected_collection_ids = Vec::new();
    let orgs = list_organizations_from_sync(&sync_data, client).await?;

    for org_conf in &config.organizations {
        if let Some(org) = orgs.iter().find(|o| o.name == org_conf.name) {
            let all_collections = list_collections_from_sync(&sync_data, client, &org.id).await?;
            let collections_to_check = if is_snippet {
                &org_conf.snippets_collections
            } else {
                &org_conf.collections
            };

            for sc in collections_to_check {
                if let Some(c) = all_collections.iter().find(|coll| &coll.name == sc) {
                    selected_collection_ids.push(c.id.clone());
                }
            }
        }
    }

    // 3. Filtrare
    ciphers.retain(|c| {
        let is_login = c.r#type == 1;
        let is_note = c.r#type == 2;

        // Dacă e snippet, vrem doar Secure Notes. Dacă e SSH, vrem doar Logins.
        if is_snippet && !is_note { return false; }
        if !is_snippet && !is_login { return false; }

        let in_personal_folder = if let Some(fid) = personal_folder_id.as_ref() {
            c.folder_id.as_ref() == Some(fid)
        } else {
            false
        };

        let in_selected_collection = if let Some(cids) = c.collection_ids.as_ref() {
            cids.iter().any(|cid| selected_collection_ids.contains(cid))
        } else {
            false
        };

        in_personal_folder || in_selected_collection
    });

    // Decriptăm numele pentru a putea fi citite în listă
    for cipher in &mut ciphers {
        if let Some(enc_name) = &cipher.name {
            if let Ok(dec_name) = decrypt_string(client, enc_name, cipher.organization_id.as_deref()) {
                cipher.name = Some(dec_name);
            }
        }
    }

    Ok(ciphers)
}

async fn fetch_and_cache_sync(client: &mut Client) -> Result<Value> {
    let api_configs = client.internal.get_api_configurations().await;
    let api_url = api_configs.api_config.base_path.clone();
    let access_token = api_configs.api_config.oauth_access_token.clone().unwrap_or_default();

    let http_client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()?;

    let sync_res = http_client.get(format!("{}/sync", api_url))
        .bearer_auth(&access_token)
        .send().await?;

    if !sync_res.status().is_success() {
        let status = sync_res.status();
        let body = sync_res.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ZbwError::SessionExpired.into());
        }
        return Err(anyhow!("Eroare server Bitwarden ({}): {}", status, body));
    }

    let sync_data: Value = sync_res.json().await?;
    
    // Salvăm în cache
    let _ = save_cached_sync(client, &sync_data);

    Ok(sync_data)
}

fn get_cache_path() -> Result<PathBuf> {
    let mut path = dirs::cache_dir().context("Nu s-a putut găsi directorul de cache")?;
    path.push("zbw");
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    path.push("vault_cache.enc");
    Ok(path)
}

fn save_cached_sync(client: &Client, data: &Value) -> Result<()> {
    let json_str = serde_json::to_string(data)?;
    let key_store = client.internal.get_key_store();
    let ctx = key_store.context();
    
    #[allow(deprecated)]
    let user_key = ctx.dangerous_get_symmetric_key(bitwarden_core::key_management::SymmetricKeyId::User)
        .map_err(|_| anyhow!("User key missing for cache encryption"))?;

    let enc_string = json_str.encrypt_with_key(user_key)
        .map_err(|e| anyhow!("Cache encryption failed: {:?}", e))?;
    
    fs::write(get_cache_path()?, enc_string.to_string())?;
    Ok(())
}

async fn load_cached_sync(client: &Client) -> Result<Value> {
    let path = get_cache_path()?;
    if !path.exists() {
        return Err(anyhow!("Cache missing"));
    }

    let enc_str = fs::read_to_string(path)?;
    let enc_string = EncString::from_str(&enc_str)
        .map_err(|e| anyhow!("Cache parsing error: {:?}", e))?;

    let key_store = client.internal.get_key_store();
    let ctx = key_store.context();
    
    #[allow(deprecated)]
    let user_key = ctx.dangerous_get_symmetric_key(bitwarden_core::key_management::SymmetricKeyId::User)
        .map_err(|_| anyhow!("User key missing for cache decryption"))?;

    let dec_bytes: Vec<u8> = enc_string.decrypt_with_key(user_key)
        .map_err(|_| anyhow!("Cache decryption failed"))?;

    let data: Value = serde_json::from_slice(&dec_bytes)?;
    Ok(data)
}

pub fn decrypt_string(client: &Client, encrypted: &str, org_id: Option<&str>) -> Result<String> {
    let key_store = client.internal.get_key_store();
    let ctx = key_store.context();

    // 1. Încercăm cheia cerută (Org sau User)
    let key_id = if let Some(oid) = org_id {
        if let Ok(uuid) = uuid::Uuid::parse_str(oid) {
            bitwarden_core::key_management::SymmetricKeyId::Organization(bitwarden_core::OrganizationId::new(uuid))
        } else {
            bitwarden_core::key_management::SymmetricKeyId::User
        }
    } else {
        bitwarden_core::key_management::SymmetricKeyId::User
    };

    if let Ok(key) = ctx.dangerous_get_symmetric_key(key_id) {
        let enc_string = EncString::from_str(encrypted)
            .map_err(|e| anyhow!("Eroare parsare: {:?}", e))?;
        if let Ok(dec) = enc_string.decrypt_with_key(key) {
            return Ok(dec);
        }
    }

    // 2. Fallback la User Key (uneori colecțiile sunt criptate cu User Key chiar dacă au Org ID)
    if let Ok(user_key) = ctx.dangerous_get_symmetric_key(bitwarden_core::key_management::SymmetricKeyId::User) {
        if let Ok(enc_string) = EncString::from_str(encrypted) {
            if let Ok(dec) = enc_string.decrypt_with_key(user_key) {
                return Ok(dec);
            }
        }
    }

    Err(anyhow!("Nu am putut decripta stringul cu nicio cheie disponibilă."))
}

pub async fn list_folders(client: &mut Client, config: &Config) -> Result<Vec<BwFolder>> {
    let sync_data = fetch_sync_with_reauth(client, config, false).await?;
    list_folders_from_sync(&sync_data, client).await
}

async fn list_folders_from_sync(sync_data: &Value, client: &Client) -> Result<Vec<BwFolder>> {
    let empty_vec = vec![];
    let folders_val = sync_data["folders"].as_array().unwrap_or(&empty_vec);
    let mut result = Vec::new();

    for f in folders_val {
        let id = f["id"].as_str().map(|s: &str| s.to_string());
        let name = if let Some(enc_name) = f["name"].as_str() {
            decrypt_string(client, enc_name, None).unwrap_or_else(|_| "Encrypted".to_string())
        } else {
            String::new()
        };
        result.push(BwFolder { id, name });
    }

    Ok(result)
}

pub async fn list_organizations(client: &mut Client, config: &Config) -> Result<Vec<BwOrganization>> {
    let sync_data = fetch_sync_with_reauth(client, config, false).await?;
    list_organizations_from_sync(&sync_data, client).await
}

async fn list_organizations_from_sync(sync_data: &Value, _client: &Client) -> Result<Vec<BwOrganization>> {
    let empty_vec = vec![];
    let orgs_val = sync_data["profile"]["organizations"].as_array().unwrap_or(&empty_vec);
    let mut result = Vec::new();

    for o in orgs_val {
        let id = o["id"].as_str().map(|s: &str| s.to_string()).unwrap_or_default();
        let name = o["name"].as_str().map(|s: &str| s.to_string()).unwrap_or_else(|| "Unknown Org".to_string());
        result.push(BwOrganization { id, name });
    }

    Ok(result)
}

pub async fn list_collections(client: &mut Client, config: &Config, org_id: &str) -> Result<Vec<BwCollection>> {
    let sync_data = fetch_sync_with_reauth(client, config, false).await?;
    list_collections_from_sync(&sync_data, client, org_id).await
}

use std::time::SystemTime;

/// Helper centralizat pentru sync cu retry pe re-autentificare
async fn fetch_sync_with_reauth(client: &mut Client, config: &Config, force_refresh: bool) -> Result<Value> {
    let mut refresh_needed = force_refresh;

    let res = if !refresh_needed {
        match load_cached_sync(client).await {
            Ok(data) => {
                // Verificăm vârsta cache-ului (Adaptive Sync)
                if let Ok(metadata) = fs::metadata(get_cache_path()?) {
                    if let Ok(modified) = metadata.modified() {
                        let now = SystemTime::now();
                        if let Ok(duration) = now.duration_since(modified) {
                            if duration.as_secs() > 3600 { // 1 oră
                                println!("🕒 Cache-ul este vechi ( > 1h). Se actualizează...");
                                refresh_needed = true;
                            }
                        }
                    }
                }
                
                if refresh_needed {
                    fetch_and_cache_sync(client).await
                } else {
                    Ok(data)
                }
            },
            Err(_) => {
                println!("🌐 Cache lipsă sau invalid. Se descarcă datele...");
                fetch_and_cache_sync(client).await
            }
        }
    } else {
        fetch_and_cache_sync(client).await
    };

    match res {
        Ok(data) => Ok(data),
        Err(e) if e.downcast_ref::<ZbwError>().map_or(false, |ee| matches!(ee, ZbwError::SessionExpired)) => {
            println!("⚠️ Sesiunea a expirat. Re-autentificare...");
            crate::auth::purge_session()?;
            let new_client = crate::auth::login_wizard(config).await?;
            *client = new_client;
            fetch_and_cache_sync(client).await
        }
        Err(e) => Err(e),
    }
}

async fn list_collections_from_sync(sync_data: &Value, client: &Client, org_id: &str) -> Result<Vec<BwCollection>> {
    let empty_vec = vec![];
    let collections_val = sync_data["collections"].as_array().unwrap_or(&empty_vec);
    let mut result = Vec::new();

    for c in collections_val {
        let oid = c["organizationId"].as_str().unwrap_or_default();
        if oid == org_id {
            let id = c["id"].as_str().unwrap_or_default().to_string();
            let name = if let Some(enc_name) = c["name"].as_str() {
                decrypt_string(client, enc_name, Some(oid)).unwrap_or_else(|_| "Encrypted Collection".to_string())
            } else {
                String::new()
            };
            result.push(BwCollection { id, name, organization_id: oid.to_string() });
        }
    }

    Ok(result)
}

