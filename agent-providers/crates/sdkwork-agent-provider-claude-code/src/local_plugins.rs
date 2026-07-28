use sdkwork_agent_plugin_core::{
    LocalPluginCatalog, LocalPluginDescriptor, LocalPluginDiscoveryRequest, LocalPluginProvider,
    LocalPluginSkillDescriptor, LocalPluginSource, LocalPluginStatus,
};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeLocalPluginProvider;

fn read_skill(path: &Path) -> Option<LocalPluginSkillDescriptor> {
    let raw = fs::read_to_string(path).ok()?;
    let name = raw.lines().find_map(|line| line.strip_prefix("name:"))
        .map(|value| value.trim().trim_matches(['"', '\'']).to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| path.parent()?.file_name()?.to_str().map(ToOwned::to_owned))?;
    let description = raw.lines().find_map(|line| line.strip_prefix("description:"))
        .map(|value| value.trim().trim_matches(['"', '\'']).to_string());
    Some(LocalPluginSkillDescriptor { skill_id: format!("skill.claude-code.{name}"), name, description, path: path.to_path_buf() })
}

impl LocalPluginProvider for ClaudeCodeLocalPluginProvider {
    fn provider_id(&self) -> &str { "provider.plugin.claude-code" }

    fn discover(&self, request: &LocalPluginDiscoveryRequest) -> LocalPluginCatalog {
        let mut catalog = LocalPluginCatalog::new(self.provider_id());
        for root in &request.roots {
            let skill_root = root.join(".claude/skills");
            let Ok(entries) = fs::read_dir(&skill_root) else { continue };
            let skills = entries.flatten().filter_map(|entry| {
                let path = if entry.path().is_dir() { entry.path().join("SKILL.md") } else { entry.path() };
                (path.file_name().is_some_and(|name| name == "SKILL.md")).then(|| read_skill(&path)).flatten()
            }).collect::<Vec<_>>();
            if skills.is_empty() { continue; }
            catalog.plugins.push(LocalPluginDescriptor {
                plugin_id: "plugin.intelligence.claude-code.local-skills".to_string(),
                name: "Claude Code local skills".to_string(),
                version: "local".to_string(),
                description: Some("Skills discovered from .claude/skills".to_string()),
                root_path: skill_root.clone(), manifest_path: skill_root,
                source: LocalPluginSource::Workspace, status: LocalPluginStatus::ProcessAdapter,
                skills, mcp_servers: Vec::new(),
            });
        }
        catalog
    }
}
