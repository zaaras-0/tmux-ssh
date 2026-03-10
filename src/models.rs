use serde::{Deserialize, Serialize};

// --- Configurația Aplicației zbw ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub email: String,
    pub server_url: String,
    pub personal_folder: String,
    pub organizations: Vec<OrgConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrgConfig {
    pub name: String,
    pub collections: Vec<String>, // Listă de nume de colecții (opțional)
}

// --- Structuri pentru Mapare Bitwarden CLI (JSON) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BwFolder {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BwOrganization {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BwCollection {
    pub id: String,
    pub name: String,
    #[serde(rename = "organizationId")]
    pub organization_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BwItem {
    pub id: String,
    pub name: String,
    pub notes: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    #[serde(rename = "organizationId")]
    pub organization_id: Option<String>,
    pub login: Option<BwLogin>,
    #[serde(rename = "type")]
    pub item_type: i32, // 1 = Login, 2 = Secure Note, etc.
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BwLogin {
    pub username: Option<String>,
    pub password: Option<String>,
    pub uris: Option<Vec<BwUri>>,
    #[serde(rename = "totp")]
    pub totp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BwUri {
    pub uri: Option<String>,
}