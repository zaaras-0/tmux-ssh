use std::fs;
use std::sync::Arc;
use std::str::FromStr;
use std::collections::HashMap;
use anyhow::{Result, anyhow, Context};
use bitwarden_core::{Client, ClientSettings};
use bitwarden_crypto::{Kdf, MasterKey, HashPurpose, SymmetricCryptoKey, BitwardenLegacyKeyBytes, EncString, KeyDecryptable, AsymmetricCryptoKey, UnsignedSharedKey};
use bitwarden_encoding::B64;
use crate::models::Config;
use crate::prompts;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SESSION_FILE: &str = "/dev/shm/zbw.session.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub user_key: String, 
    pub org_keys: HashMap<String, String>, // org_id -> decrypted_key_b64
}

#[derive(Debug)]
pub struct MemoryTokens {
    pub access_token: String,
}

#[async_trait::async_trait]
impl bitwarden_core::client::internal::ClientManagedTokens for MemoryTokens {
    async fn get_access_token(&self) -> Option<String> {
        Some(self.access_token.clone())
    }
}

pub async fn get_client(config: &Config) -> Result<Client> {
    let mut settings = ClientSettings::default();
    let (api_url, identity_url) = if config.server_url.is_empty() || config.server_url.contains("bitwarden.com") {
        ("https://api.bitwarden.com".to_string(), "https://identity.bitwarden.com".to_string())
    } else {
        let base = config.server_url.trim_end_matches('/');
        (format!("{}/api", base), format!("{}/identity", base))
    };
    
    settings.api_url = api_url;
    settings.identity_url = identity_url;

    if let Ok(session) = load_session() {
        let tokens = Arc::new(MemoryTokens { access_token: session.access_token });
        let client = Client::new_with_client_tokens(Some(settings), tokens);
        
        // Restaurăm cheia utilizatorului
        if let Ok(user_key_bytes) = B64::try_from(session.user_key.as_str()) {
            let legacy_bytes = BitwardenLegacyKeyBytes::from(user_key_bytes.as_bytes().to_vec());
            if let Ok(user_key) = SymmetricCryptoKey::try_from(&legacy_bytes) {
                use bitwarden_core::key_management::SymmetricKeyId;
                let key_store = client.internal.get_key_store();
                #[allow(deprecated)]
                let _ = key_store.context_mut().set_symmetric_key(SymmetricKeyId::User, user_key);
            }
        }

        // Restaurăm cheile organizațiilor
        for (oid, ok_b64) in session.org_keys {
            if let Ok(oid_uuid) = uuid::Uuid::parse_str(&oid) {
                if let Ok(ok_bytes) = B64::try_from(ok_b64.as_str()) {
                    let legacy = BitwardenLegacyKeyBytes::from(ok_bytes.as_bytes().to_vec());
                    if let Ok(ok_dec) = SymmetricCryptoKey::try_from(&legacy) {
                        use bitwarden_core::key_management::SymmetricKeyId;
                        let key_store = client.internal.get_key_store();
                        #[allow(deprecated)]
                        let _ = key_store.context_mut().set_symmetric_key(SymmetricKeyId::Organization(bitwarden_core::OrganizationId::new(oid_uuid)), ok_dec);
                    }
                }
            }
        }
        
        return Ok(client);
    }

    login_wizard(config).await
}

pub async fn login_wizard(config: &Config) -> Result<Client> {
    println!("🔑 Autentificare Bitwarden pentru: {}", config.email);
    let password = prompts::ask_password("Master Password")?;

    let (api_url, identity_url) = if config.server_url.is_empty() || config.server_url.contains("bitwarden.com") {
        ("https://api.bitwarden.com".to_string(), "https://identity.bitwarden.com".to_string())
    } else {
        let base = config.server_url.trim_end_matches('/');
        (format!("{}/api", base), format!("{}/identity", base))
    };

    let http_client = reqwest::Client::new();

    // 1. Prelogin Manual
    let prelogin_res = http_client.post(format!("{}/accounts/prelogin", api_url))
        .json(&serde_json::json!({ "email": config.email }))
        .send().await?;

    let prelogin_data: Value = prelogin_res.json().await?;
    
    let kdf_type = prelogin_data["kdf"].as_i64().unwrap_or(0);
    let kdf_iterations = prelogin_data["kdfIterations"].as_u64().unwrap_or(100000);
    
    let kdf = match kdf_type {
        0 => Kdf::PBKDF2 { iterations: std::num::NonZeroU32::new(kdf_iterations as u32).unwrap() },
        1 => Kdf::Argon2id {
            iterations: std::num::NonZeroU32::new(prelogin_data["kdfIterations"].as_u64().unwrap_or(3) as u32).unwrap(),
            memory: std::num::NonZeroU32::new(prelogin_data["kdfMemory"].as_u64().unwrap_or(65536) as u32).unwrap(),
            parallelism: std::num::NonZeroU32::new(prelogin_data["kdfParallelism"].as_u64().unwrap_or(4) as u32).unwrap(),
        },
        _ => return Err(anyhow!("KDF nesuportat: {}", kdf_type)),
    };

    // 2. Generare Hash Parolă
    let master_key = MasterKey::derive(&password, &config.email, &kdf)?;
    let password_hash = master_key.derive_master_key_hash(password.as_bytes(), HashPurpose::ServerAuthorization);
    let password_hash_b64 = password_hash.to_string();

    // 3. Login (connect/token)
    let login_params = [
        ("grant_type", "password"),
        ("scope", "api offline_access"),
        ("client_id", "web"),
        ("deviceType", "14"),
        ("deviceIdentifier", "b86dd6ab-4265-4ddf-a7f1-eb28d5677f33"),
        ("deviceName", "zbw-cli"),
        ("username", &config.email),
        ("password", &password_hash_b64),
    ];

    let login_res = http_client.post(format!("{}/connect/token", identity_url))
        .form(&login_params)
        .send().await?;

    if !login_res.status().is_success() {
        return Err(anyhow!("Autentificare eșuată. Verifică parola."));
    }

    let token_data: Value = login_res.json().await?;
    let access_token = token_data["access_token"].as_str().context("Nu am primit access_token")?.to_string();

    // 4. Preluare și Decriptare Cheie Utilizator
    let sync_res = http_client.get(format!("{}/sync", api_url))
        .bearer_auth(&access_token)
        .send().await?;
    
    let sync_data: Value = sync_res.json().await?;
    let enc_user_key = sync_data["profile"]["key"].as_str()
        .or(token_data["Key"].as_str())
        .or(token_data["key"].as_str())
        .context("Nu am găsit cheia utilizatorului pe server")?;

    let user_key_decrypted = master_key.decrypt_user_key(enc_user_key.parse()?)?;

    // 4.1 Preluare și Decriptare Cheie Privată (pentru RSA Org Keys)
    let enc_priv_key = sync_data["profile"]["privateKey"].as_str()
        .or(sync_data["profile"]["encryptedPrivateKey"].as_str())
        .or(token_data["PrivateKey"].as_str());
    
    let priv_key = if let Some(epk) = enc_priv_key {
        if let Ok(enc_string) = EncString::from_str(epk) {
            let pk_bytes_res: Result<Vec<u8>, _> = enc_string.decrypt_with_key(&user_key_decrypted);
            if let Ok(pk_bytes) = pk_bytes_res {
                AsymmetricCryptoKey::from_der(&pk_bytes.into()).ok()
            } else { None }
        } else { None }
    } else { None };

    // 4.2 Decriptare Chei Organizație
    let mut org_keys = HashMap::new();
    if let Some(orgs_list) = sync_data["profile"]["organizations"].as_array() {
        for org in orgs_list {
            if let (Some(oid), Some(enc_ok), Some(name)) = (org["id"].as_str(), org["key"].as_str(), org["name"].as_str()) {
                let ok_dec_res = if enc_ok.starts_with("4.") {
                    if let (Ok(usk), Some(pk)) = (UnsignedSharedKey::from_str(enc_ok), &priv_key) {
                        usk.decapsulate_key_unsigned(pk)
                    } else {
                        Err(bitwarden_crypto::CryptoError::InvalidKey)
                    }
                } else if let Ok(enc_string) = EncString::from_str(enc_ok) {
                    let bytes_res: Result<Vec<u8>, _> = enc_string.decrypt_with_key(&user_key_decrypted);
                    bytes_res.map_err(|e| bitwarden_crypto::CryptoError::from(e))
                        .and_then(|b| SymmetricCryptoKey::try_from(&BitwardenLegacyKeyBytes::from(b)))
                } else {
                    Err(bitwarden_crypto::CryptoError::InvalidKey)
                };

                if let Ok(ok_dec) = ok_dec_res {
                    org_keys.insert(oid.to_string(), ok_dec.to_base64().to_string());
                    println!(" [DEBUG] Cheie decriptată pentru: {}", name);
                }
            }
        }
    }

    // 5. Salvare Sesiune
    let session = SessionData {
        access_token: access_token.clone(),
        refresh_token: token_data["refresh_token"].as_str().map(|s| s.to_string()),
        user_key: user_key_decrypted.to_base64().to_string(),
        org_keys: org_keys.clone(),
    };
    save_session(&session)?;

    // 6. Configurare Client SDK și Injectare Chei
    let mut settings = ClientSettings::default();
    settings.api_url = api_url;
    settings.identity_url = identity_url;
    
    let tokens = Arc::new(MemoryTokens { access_token });
    let client = Client::new_with_client_tokens(Some(settings), tokens);
    
    {
        use bitwarden_core::key_management::SymmetricKeyId;
        let key_store = client.internal.get_key_store();
        
        #[allow(deprecated)]
        let _ = key_store.context_mut().set_symmetric_key(SymmetricKeyId::User, user_key_decrypted);

        for (oid, ok_b64) in org_keys {
            if let Ok(oid_uuid) = uuid::Uuid::parse_str(&oid) {
                if let Ok(ok_bytes) = B64::try_from(ok_b64.as_str()) {
                    let legacy = BitwardenLegacyKeyBytes::from(ok_bytes.as_bytes().to_vec());
                    if let Ok(ok_dec) = SymmetricCryptoKey::try_from(&legacy) {
                        #[allow(deprecated)]
                        let _ = key_store.context_mut().set_symmetric_key(SymmetricKeyId::Organization(bitwarden_core::OrganizationId::new(oid_uuid)), ok_dec);
                    }
                }
            }
        }
    }

    println!("✅ Autentificare reușită și date decriptate!");
    Ok(client)
}

pub fn purge_session() -> Result<()> {
    let _ = fs::remove_file(SESSION_FILE);
    println!("🔐 Sesiune eliminată.");
    Ok(())
}

pub fn get_active_session() -> Result<String> {
    let session = load_session()?;
    Ok(session.access_token)
}

pub fn check_status(_session: &Option<String>) -> Result<Value> {
    Ok(serde_json::json!({"status": "Authenticated"}))
}

fn load_session() -> Result<SessionData> {
    let content = fs::read_to_string(SESSION_FILE)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_session(session: &SessionData) -> Result<()> {
    let content = serde_json::to_string(session)?;
    fs::write(SESSION_FILE, content)?;
    Ok(())
}
