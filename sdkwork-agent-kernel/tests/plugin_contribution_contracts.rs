//! Contract tests: plugin contribution points.
//!
//! Plugins declare contribution points (provider / tool / hook / stream /
//! memory / skill) that the kernel registry surfaces for discovery.
//! Contribution ids must be unique across all plugins in one registry;
//! the kernel rejects duplicate declarations at registration time. This
//! mirrors the provider-binding surface of `sdkwork-kernel-plugins`
//! `KernelPluginManifest.provider_ids` at the kernel SPI level.

use sdkwork_agent_kernel::{
    KernelResult, Plugin, PluginContext, PluginContribution, PluginContributionKind,
    PluginMetadata, PluginRegistry, PluginState, ProviderHealth, ProviderManifest,
};

/// Minimal plugin whose only behavior is declaring contributions.
struct ContributingPlugin {
    plugin_id: String,
    contributions: Vec<PluginContribution>,
}

impl ContributingPlugin {
    fn new(plugin_id: &str, contributions: Vec<PluginContribution>) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            contributions,
        }
    }
}

impl Plugin for ContributingPlugin {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            &self.plugin_id,
            "test",
            "contributing-plugin",
            "1.0.0",
            vec!["plugin.contributions".to_string()],
        )
    }

    fn initialize(&mut self, _context: &PluginContext) -> KernelResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> KernelResult<()> {
        Ok(())
    }

    fn activate(&mut self) -> KernelResult<()> {
        Ok(())
    }

    fn deactivate(&mut self) -> KernelResult<()> {
        Ok(())
    }

    fn health_check(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn state(&self) -> PluginState {
        PluginState::Active
    }

    fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    fn contributions(&self) -> Vec<PluginContribution> {
        self.contributions.clone()
    }
}

fn metadata(plugin_id: &str) -> PluginMetadata {
    PluginMetadata::new(plugin_id, "1.0.0", "contributing-plugin")
}

#[test]
fn contributions_are_discoverable_after_registration() {
    let registry = PluginRegistry::new();
    registry
        .register(
            Box::new(ContributingPlugin::new(
                "plugin.contribution.a",
                vec![
                    PluginContribution::provider("provider.acme", "acme model"),
                    PluginContribution::tool("tool.plugin.search", "search tool"),
                    PluginContribution::hook("hook.plugin.policy", "policy hook"),
                ],
            )),
            metadata("plugin.contribution.a"),
            PluginContext::new("plugin.contribution.a", "1.0.0", "runtime-1"),
        )
        .expect("registration succeeds");

    let contributions = registry.contributions();
    assert_eq!(contributions.len(), 3);
    assert!(contributions
        .iter()
        .any(
            |(plugin_id, contribution)| plugin_id == "plugin.contribution.a"
                && contribution.contribution_id == "provider.acme"
                && contribution.kind == PluginContributionKind::Provider
        ));
}

#[test]
fn plugins_without_declarations_expose_no_contributions() {
    let registry = PluginRegistry::new();
    registry
        .register(
            Box::new(ContributingPlugin::new("plugin.plain", Vec::new())),
            metadata("plugin.plain"),
            PluginContext::new("plugin.plain", "1.0.0", "runtime-1"),
        )
        .expect("registration succeeds");

    assert!(registry.contributions().is_empty());
    assert_eq!(
        registry
            .contributions_of_kind(PluginContributionKind::Tool)
            .len(),
        0
    );
}

#[test]
fn contributions_can_be_filtered_by_kind() {
    let registry = PluginRegistry::new();
    registry
        .register(
            Box::new(ContributingPlugin::new(
                "plugin.mixed",
                vec![
                    PluginContribution::provider("provider.acme", "acme model"),
                    PluginContribution::memory("memory.plugin.growth", "growth memory"),
                    PluginContribution::skill("skill.plugin.review", "review skill"),
                ],
            )),
            metadata("plugin.mixed"),
            PluginContext::new("plugin.mixed", "1.0.0", "runtime-1"),
        )
        .expect("registration succeeds");

    let tools = registry.contributions_of_kind(PluginContributionKind::Tool);
    assert!(tools.is_empty());

    let skills = registry.contributions_of_kind(PluginContributionKind::Skill);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].1.contribution_id, "skill.plugin.review");
    assert_eq!(skills[0].0, "plugin.mixed");
}

#[test]
fn contribution_id_resolves_to_owning_plugin() {
    let registry = PluginRegistry::new();
    registry
        .register(
            Box::new(ContributingPlugin::new(
                "plugin.owner",
                vec![PluginContribution::stream(
                    "stream.plugin.events",
                    "event stream",
                )],
            )),
            metadata("plugin.owner"),
            PluginContext::new("plugin.owner", "1.0.0", "runtime-1"),
        )
        .expect("registration succeeds");

    assert_eq!(
        registry
            .plugin_contributing("stream.plugin.events")
            .as_deref(),
        Some("plugin.owner")
    );
    assert!(registry
        .plugin_contributing("unknown.contribution")
        .is_none());
}

#[test]
fn duplicate_contribution_id_across_plugins_is_rejected() {
    let registry = PluginRegistry::new();
    registry
        .register(
            Box::new(ContributingPlugin::new(
                "plugin.first",
                vec![PluginContribution::provider(
                    "provider.shared",
                    "first provider",
                )],
            )),
            metadata("plugin.first"),
            PluginContext::new("plugin.first", "1.0.0", "runtime-1"),
        )
        .expect("first registration succeeds");

    let result = registry.register(
        Box::new(ContributingPlugin::new(
            "plugin.second",
            vec![PluginContribution::provider(
                "provider.shared",
                "second provider",
            )],
        )),
        metadata("plugin.second"),
        PluginContext::new("plugin.second", "1.0.0", "runtime-1"),
    );
    assert!(
        result.is_err(),
        "duplicate contribution id must be rejected"
    );
    assert_eq!(
        registry.contributions().len(),
        1,
        "second plugin must not be registered"
    );
}

#[test]
fn duplicate_plugin_id_is_still_rejected() {
    let registry = PluginRegistry::new();
    let plugin = ContributingPlugin::new(
        "plugin.dup",
        vec![PluginContribution::tool("tool.plugin.x", "x")],
    );
    registry
        .register(
            Box::new(plugin),
            metadata("plugin.dup"),
            PluginContext::new("plugin.dup", "1.0.0", "runtime-1"),
        )
        .expect("first registration succeeds");

    let result = registry.register(
        Box::new(ContributingPlugin::new(
            "plugin.dup",
            vec![PluginContribution::tool("tool.plugin.y", "y")],
        )),
        metadata("plugin.dup"),
        PluginContext::new("plugin.dup", "1.0.0", "runtime-1"),
    );
    assert!(result.is_err());
}

#[test]
fn kind_round_trips_through_strings() {
    for kind in [
        PluginContributionKind::Provider,
        PluginContributionKind::Tool,
        PluginContributionKind::Hook,
        PluginContributionKind::Stream,
        PluginContributionKind::Memory,
        PluginContributionKind::Skill,
    ] {
        assert_eq!(PluginContributionKind::from_str(kind.as_str()), Some(kind));
    }
    assert_eq!(PluginContributionKind::from_str("unknown"), None);
    assert_eq!(PluginContributionKind::Hook.as_str(), "hook");
}

#[test]
fn contribution_constructors_set_kind() {
    assert_eq!(
        PluginContribution::provider("p", "d").kind,
        PluginContributionKind::Provider
    );
    assert_eq!(
        PluginContribution::tool("t", "d").kind,
        PluginContributionKind::Tool
    );
    assert_eq!(
        PluginContribution::hook("h", "d").kind,
        PluginContributionKind::Hook
    );
    assert_eq!(
        PluginContribution::stream("s", "d").kind,
        PluginContributionKind::Stream
    );
    assert_eq!(
        PluginContribution::memory("m", "d").kind,
        PluginContributionKind::Memory
    );
    assert_eq!(
        PluginContribution::skill("s", "d").kind,
        PluginContributionKind::Skill
    );
}
