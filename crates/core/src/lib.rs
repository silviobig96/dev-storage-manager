use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    #[serde(rename = "macos")]
    MacOs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppHealth {
    pub app_name: String,
    pub app_version: String,
    pub platform: Platform,
    pub database_schema_version: u32,
}

#[cfg(test)]
mod tests {
    use super::{AppHealth, Platform};

    #[test]
    fn health_contract_serializes_with_stable_field_names() {
        let health = AppHealth {
            app_name: "dev-storage-manager".into(),
            app_version: "0.1.0".into(),
            platform: Platform::MacOs,
            database_schema_version: 1,
        };

        assert_eq!(
            serde_json::to_value(health).unwrap(),
            serde_json::json!({
                "appName": "dev-storage-manager",
                "appVersion": "0.1.0",
                "platform": "macos",
                "databaseSchemaVersion": 1
            })
        );
    }
}
