use sdkwork_agent_plugin_core::{
    LocalPluginCatalog, LocalPluginDescriptor, LocalPluginDiscoveryRequest, LocalPluginLoadError,
    LocalPluginLoadErrorKind, LocalPluginProvider, LocalPluginSkillDescriptor, LocalPluginSource,
    LocalPluginStatus,
};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct CodexPluginManifest {
    name: String,
    version: String,
    description: Option<String>,
    skills: Option<String>,
    #[serde(rename = "mcpServers")]
    mcp_servers: Option<String>,
}

/// Discovers Codex plugins without loading executable code. This follows the
/// Codex `.codex-plugin/plugin.json` contract and reports malformed entries as
/// data so valid plugins remain visible.
#[derive(Debug, Clone, Default)]
pub struct CodexLocalPluginProvider;

impl CodexLocalPluginProvider {
    pub fn new() -> Self {
        Self
    }

    fn discover_manifest(&self, manifest_path: &Path, catalog: &mut LocalPluginCatalog) {
        let Some(root_path) = manifest_path.parent().and_then(Path::parent) else {
            return;
        };
        let raw = match fs::read_to_string(manifest_path) {
            Ok(value) => value,
            Err(error) => {
                catalog.errors.push(LocalPluginLoadError {
                    provider_id: self.provider_id().to_string(),
                    path: Some(manifest_path.to_path_buf()),
                    kind: LocalPluginLoadErrorKind::PermissionDenied,
                    message: error.to_string(),
                });
                return;
            }
        };
        let manifest = match serde_json::from_str::<CodexPluginManifest>(&raw) {
            Ok(value) if !value.name.trim().is_empty() && !value.version.trim().is_empty() => value,
            Ok(_) => {
                catalog
                    .errors
                    .push(self.invalid_manifest(manifest_path, "name and version are required"));
                return;
            }
            Err(error) => {
                catalog
                    .errors
                    .push(self.invalid_manifest(manifest_path, &error.to_string()));
                return;
            }
        };

        let skills = self.discover_skills(root_path, manifest.skills.as_deref(), catalog);
        let mcp_servers =
            self.discover_mcp_servers(root_path, manifest.mcp_servers.as_deref(), catalog);
        catalog.plugins.push(LocalPluginDescriptor {
            plugin_id: format!("plugin.intelligence.codex.{}", manifest.name),
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            root_path: root_path.to_path_buf(),
            manifest_path: manifest_path.to_path_buf(),
            source: LocalPluginSource::User,
            status: LocalPluginStatus::ProcessAdapter,
            skills,
            mcp_servers,
        });
    }

    fn invalid_manifest(&self, path: &Path, message: &str) -> LocalPluginLoadError {
        LocalPluginLoadError {
            provider_id: self.provider_id().to_string(),
            path: Some(path.to_path_buf()),
            kind: LocalPluginLoadErrorKind::InvalidManifest,
            message: message.to_string(),
        }
    }

    fn discover_skills(
        &self,
        root: &Path,
        configured: Option<&str>,
        catalog: &mut LocalPluginCatalog,
    ) -> Vec<LocalPluginSkillDescriptor> {
        let skill_root = configured
            .map(|value| root.join(value.trim_start_matches("./")))
            .unwrap_or_else(|| root.join("skills"));
        let mut candidates = Vec::new();
        if skill_root.is_file() {
            candidates.push(skill_root);
        } else if let Ok(entries) = fs::read_dir(&skill_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill = path.join("SKILL.md");
                    if skill.is_file() {
                        candidates.push(skill);
                    }
                } else if path.file_name().is_some_and(|name| name == "SKILL.md") {
                    candidates.push(path);
                }
            }
        }
        candidates.sort();
        candidates
            .into_iter()
            .filter_map(|path| self.parse_skill(&path, catalog))
            .collect()
    }

    fn parse_skill(
        &self,
        path: &Path,
        catalog: &mut LocalPluginCatalog,
    ) -> Option<LocalPluginSkillDescriptor> {
        let raw = match fs::read_to_string(path) {
            Ok(value) => value,
            Err(error) => {
                catalog.errors.push(LocalPluginLoadError {
                    provider_id: self.provider_id().to_string(),
                    path: Some(path.to_path_buf()),
                    kind: LocalPluginLoadErrorKind::InvalidSkill,
                    message: error.to_string(),
                });
                return None;
            }
        };
        let mut name = None;
        let mut description = None;
        for line in raw.lines().take(32) {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim().trim_matches(['"', '\'']);
            match key.trim() {
                "name" => name = Some(value.to_string()),
                "description" => description = Some(value.to_string()),
                _ => {}
            }
        }
        let name = name
            .filter(|value| !value.is_empty())
            .or_else(|| path.parent()?.file_name()?.to_str().map(ToOwned::to_owned))?;
        Some(LocalPluginSkillDescriptor {
            skill_id: format!("skill.codex.{name}"),
            name,
            description,
            path: path.to_path_buf(),
        })
    }

    fn discover_mcp_servers(
        &self,
        root: &Path,
        configured: Option<&str>,
        catalog: &mut LocalPluginCatalog,
    ) -> Vec<String> {
        let Some(configured) = configured else {
            return Vec::new();
        };
        let path = root.join(configured.trim_start_matches("./"));
        let raw = match fs::read_to_string(&path) {
            Ok(value) => value,
            Err(error) => {
                catalog.errors.push(LocalPluginLoadError {
                    provider_id: self.provider_id().to_string(),
                    path: Some(path),
                    kind: LocalPluginLoadErrorKind::InvalidManifest,
                    message: error.to_string(),
                });
                return Vec::new();
            }
        };
        let value = match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(value) => value,
            Err(error) => {
                catalog
                    .errors
                    .push(self.invalid_manifest(&path, &error.to_string()));
                return Vec::new();
            }
        };
        value
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl LocalPluginProvider for CodexLocalPluginProvider {
    fn provider_id(&self) -> &str {
        "provider.plugin.codex"
    }

    fn discover(&self, request: &LocalPluginDiscoveryRequest) -> LocalPluginCatalog {
        let mut catalog = LocalPluginCatalog::new(self.provider_id());
        let mut manifests = Vec::new();
        for root in &request.roots {
            let direct = root.join(".codex-plugin/plugin.json");
            if direct.is_file() {
                manifests.push(direct);
            }
            if let Ok(entries) = fs::read_dir(root) {
                for entry in entries.flatten() {
                    let candidate = entry.path().join(".codex-plugin/plugin.json");
                    if candidate.is_file() {
                        manifests.push(candidate);
                    }
                }
            }
        }
        manifests.sort();
        manifests.dedup();
        for manifest in manifests {
            self.discover_manifest(&manifest, &mut catalog);
        }
        catalog
    }
}
