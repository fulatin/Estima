use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub uri: String,
    pub name: String,
    pub plugin_type: String,
    pub bypass: bool,
    pub has_ui: bool,
}

fn main() {
    let info = PluginInfo {
        id: "test-123".to_string(),
        uri: "http://test.plugin".to_string(),
        name: "Test Plugin".to_string(),
        plugin_type: "eq".to_string(),
        bypass: false,
        has_ui: true,
    };
    
    let json = serde_json::to_string_pretty(&info).unwrap();
    println!("Serialized JSON:\n{}", json);
}
