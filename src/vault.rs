use anyhow::{Result, anyhow, Context};
use bitwarden_core::Client;
use bitwarden_api_api::models::CipherDetailsResponseModel;
use bitwarden_core::key_management::SymmetricKeyId;
use bitwarden_crypto::KeyDecryptable;
use bitwarden_crypto::EncString;
use serde_json::Value;
use std::str::FromStr;
use crate::models::*;

/// Extrage toate item-urile filtrate conform configurației (Personal + Orgs)
pub async fn fetch_filtered_items(_config: &Config, client: &mut Client) -> Result<Vec<CipherDetailsResponseModel>> {
    let api_configs = client.internal.get_api_configurations().await;
    let api_url = api_configs.api_config.base_path.clone();
    let access_token = api_configs.api_config.oauth_access_token.clone().unwrap_or_default();

    let http_client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()?;

    println!("🔍 Se încarcă datele din seif...");
    let sync_res = http_client.get(format!("{}/sync", api_url))
        .bearer_auth(&access_token)
        .send().await?;
    
    let sync_data: Value = sync_res.json().await?;

    let ciphers_val = sync_data["ciphers"].as_array().context("Nu am găsit ciphers în sync")?;
    let mut ciphers: Vec<CipherDetailsResponseModel> = serde_json::from_value(serde_json::Value::Array(ciphers_val.clone()))?;

    // 1. Decriptăm folderele pentru a găsi ID-ul folderului personal ales
    let folders = list_folders(client).await?;
    let personal_folder_id = folders.iter()
        .find(|f| f.name == _config.personal_folder)
        .and_then(|f| f.id.clone());

    // 2. Extragem ID-urile colecțiilor selectate din organizații
    let mut selected_collection_ids = Vec::new();
    let orgs = list_organizations(client).await?;
    
    for org_conf in &_config.organizations {
        if let Some(org) = orgs.iter().find(|o| o.name == org_conf.name) {
            let all_collections = list_collections(client, &org.id).await?;
            for sc in &org_conf.collections {
                if let Some(c) = all_collections.iter().find(|coll| &coll.name == sc) {
                    selected_collection_ids.push(c.id.clone());
                }
            }
        }
    }

    // 3. Filtrare
    ciphers.retain(|c| {
        let is_login = c.r#type == Some(bitwarden_api_api::models::CipherType::Login);
        
        let in_personal_folder = if let Some(fid) = personal_folder_id.as_ref() {
            c.folder_id.map(|u| u.to_string()) == Some(fid.clone())
        } else {
            false
        };

        let in_selected_collection = if let Some(cids) = c.collection_ids.as_ref() {
            cids.iter().any(|cid| selected_collection_ids.contains(&cid.to_string()))
        } else {
            false
        };

        is_login && (in_personal_folder || in_selected_collection)
    });

    // Decriptăm numele pentru a putea fi citite în listă
    for cipher in &mut ciphers {
        if let Some(enc_name) = &cipher.name {
            if let Ok(dec_name) = decrypt_string(client, enc_name, cipher.organization_id.map(|u| u.to_string()).as_deref()) {
                cipher.name = Some(dec_name);
            }
        }
    }

    Ok(ciphers)
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

pub async fn list_folders(client: &Client) -> Result<Vec<BwFolder>> {
    let api_configs = client.internal.get_api_configurations().await;
    let api_url = api_configs.api_config.base_path.clone();
    let access_token = api_configs.api_config.oauth_access_token.clone().unwrap_or_default();

    let http_client = reqwest::Client::builder().use_rustls_tls().build()?;
    let sync_res = http_client.get(format!("{}/sync", api_url))
        .bearer_auth(&access_token)
        .send().await?;
    let sync_data: Value = sync_res.json().await?;

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

pub async fn list_organizations(client: &Client) -> Result<Vec<BwOrganization>> {
    let api_configs = client.internal.get_api_configurations().await;
    let api_url = api_configs.api_config.base_path.clone();
    let access_token = api_configs.api_config.oauth_access_token.clone().unwrap_or_default();

    let http_client = reqwest::Client::builder().use_rustls_tls().build()?;
    let sync_res = http_client.get(format!("{}/sync", api_url))
        .bearer_auth(&access_token)
        .send().await?;
    let sync_data: Value = sync_res.json().await?;

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

pub async fn list_collections(client: &Client, org_id: &str) -> Result<Vec<BwCollection>> {
    let api_configs = client.internal.get_api_configurations().await;
    let api_url = api_configs.api_config.base_path.clone();
    let access_token = api_configs.api_config.oauth_access_token.clone().unwrap_or_default();

    let http_client = reqwest::Client::builder().use_rustls_tls().build()?;
    let sync_res = http_client.get(format!("{}/sync", api_url))
        .bearer_auth(&access_token)
        .send().await?;
    let sync_data: Value = sync_res.json().await?;

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
