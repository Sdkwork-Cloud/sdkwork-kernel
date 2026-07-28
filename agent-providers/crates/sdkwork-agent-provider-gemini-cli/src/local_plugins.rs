use sdkwork_agent_plugin_core::{
    LocalPluginCatalog, LocalPluginDescriptor, LocalPluginDiscoveryRequest, LocalPluginProvider,
    LocalPluginSkillDescriptor, LocalPluginSource, LocalPluginStatus,
};
use serde_json::Value;
use std::fs;

#[derive(Debug, Clone, Default)]
pub struct GeminiCliLocalPluginProvider;

impl LocalPluginProvider for GeminiCliLocalPluginProvider {
    fn provider_id(&self) -> &str { "provider.plugin.gemini-cli" }

    fn discover(&self, request: &LocalPluginDiscoveryRequest) -> LocalPluginCatalog {
        let mut catalog = LocalPluginCatalog::new(self.provider_id());
        for root in &request.roots {
            let manifest_path = root.join("gemini-extension.json");
            let Ok(raw) = fs::read_to_string(&manifest_path) else { continue };
            let Ok(value) = serde_json::from_str::<Value>(&raw) else { continue };
            let Some(name) = value.get("name").and_then(Value::as_str) else { continue };
            let skill_root = root.join("skills");
            let skills = fs::read_dir(&skill_root).ok().into_iter().flatten().flatten().filter_map(|entry| {
                let path = entry.path().join("SKILL.md");
                let skill_name = path.parent()?.file_name()?.to_str()?.to_string();
                path.is_file().then(|| LocalPluginSkillDescriptor { skill_id: format!("skill.gemini.{skill_name}"), name: skill_name, description: None, path })
            }).collect::<Vec<_>>();
            let mcp_servers = value.get("mcpServers").and_then(Value::as_object).map(|object| object.keys().cloned().collect()).unwrap_or_default();
            catalog.plugins.push(LocalPluginDescriptor {
                plugin_id: format!("plugin.intelligence.gemini-cli.{name}"), name: name.to_string(),
                version: value.get("version").and_then(Value::as_str).unwrap_or("0.0.0").to_string(),
                description: value.get("description").and_then(Value::as_str).map(ToOwned::to_owned),
                root_path: root.clone(), manifest_path, source: LocalPluginSource::User,
                status: LocalPluginStatus::ProcessAdapter, skills, mcp_servers,
            });
        }
        catalog
    }
}
