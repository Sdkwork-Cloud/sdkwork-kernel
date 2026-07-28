use sdkwork_agent_plugin_core::{
    LocalPluginCatalog, LocalPluginDescriptor, LocalPluginDiscoveryRequest, LocalPluginProvider,
    LocalPluginSkillDescriptor, LocalPluginSource, LocalPluginStatus,
};
use std::fs;

#[derive(Debug, Clone, Default)]
pub struct OpenCodeLocalPluginProvider;

impl LocalPluginProvider for OpenCodeLocalPluginProvider {
    fn provider_id(&self) -> &str { "provider.plugin.opencode" }

    fn discover(&self, request: &LocalPluginDiscoveryRequest) -> LocalPluginCatalog {
        let mut catalog = LocalPluginCatalog::new(self.provider_id());
        for root in &request.roots {
            let command_root = root.join(".opencode/commands");
            let Ok(entries) = fs::read_dir(&command_root) else { continue };
            let skills = entries.flatten().filter_map(|entry| {
                let path = entry.path();
                if path.extension().is_none_or(|ext| ext != "md") { return None; }
                let name = path.file_stem()?.to_str()?.to_string();
                let description = fs::read_to_string(&path).ok().and_then(|raw| raw.lines().find(|line| !line.trim().is_empty()).map(ToOwned::to_owned));
                Some(LocalPluginSkillDescriptor { skill_id: format!("skill.opencode.{name}"), name, description, path })
            }).collect::<Vec<_>>();
            if skills.is_empty() { continue; }
            catalog.plugins.push(LocalPluginDescriptor {
                plugin_id: "plugin.intelligence.opencode.local-commands".to_string(),
                name: "OpenCode local commands".to_string(), version: "local".to_string(),
                description: Some("Commands discovered from .opencode/commands".to_string()),
                root_path: command_root.clone(), manifest_path: command_root,
                source: LocalPluginSource::Workspace, status: LocalPluginStatus::ProcessAdapter,
                skills, mcp_servers: Vec::new(),
            });
        }
        catalog
    }
}
