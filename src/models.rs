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
    pub collections: Vec<String>,
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
