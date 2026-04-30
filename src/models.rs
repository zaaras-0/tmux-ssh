use serde::{Deserialize, Serialize};

// --- Configurația Aplicației zbw ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub email: String,
    pub server_url: String,
    pub personal_folder: String,
    pub personal_snippets_folder: String,
    pub organizations: Vec<OrgConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrgConfig {
    pub name: String,
    pub collections: Vec<String>,
    pub snippets_collections: Vec<String>,
}

// --- Structuri Helper pentru Skim/UI ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BwFolder {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BwOrganization {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BwCollection {
    pub id: String,
    pub name: String,
    pub organization_id: String,
}

// --- Modele pentru Sync ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BwCipher {
    pub id: String,
    pub organization_id: Option<String>,
    pub folder_id: Option<String>,
    pub r#type: i32,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub login: Option<BwCipherLogin>,
    #[serde(default)]
    pub collection_ids: Option<Vec<String>>,
    #[serde(default)]
    pub fields: Option<Vec<BwField>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BwField {
    pub name: Option<String>,
    pub value: Option<String>,
    pub r#type: i32, // 0 = Text, 1 = Hidden, 2 = Boolean
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BwCipherLogin {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub uris: Option<Vec<BwCipherUri>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BwCipherUri {
    pub uri: Option<String>,
}
