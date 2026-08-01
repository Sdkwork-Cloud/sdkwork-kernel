//! Contract tests for the SKILL.md-aligned skill SPI.
//!
//! Skills ship as markdown units with YAML frontmatter and three-layer
//! progressive disclosure (`body` resident, `references`/`scripts` on
//! demand, `assets` never loaded). Visibility overrides control model
//! invocation, aligning with the agent skill ecosystems and
//! `SDKWORK_WORKSPACE_SPEC` `.sdkwork/skills/<name>/SKILL.md`.

use sdkwork_agent_kernel::{
    parse_skill_markdown_frontmatter, AgentSkillDescriptor, AgentSkillInvocationMode,
    AgentSkillProvider, AgentSkillRequest, AgentSkillResult, KernelErrorKind, KernelResult,
    ProviderHealth, ProviderManifest, SideEffectLevel, SkillContentFile, SkillContentLayer,
    SkillContentLayout, SkillMarkdownFrontmatter, SkillVisibility,
};

const SKILL_MARKDOWN: &str = r#"---
name: review-code
description: Review Rust code for correctness and style before commit.
version: 1.2.0
license: MIT
argument-hint: "path to the code to review"
allowed-tools: Read, Grep, Glob
disallowed-tools: Bash, Write
paths: src/, tests/
---

Review the given Rust code. Use the references for the local conventions.
"#;

#[test]
fn skill_markdown_frontmatter_parses_all_fields() {
    let frontmatter =
        parse_skill_markdown_frontmatter(SKILL_MARKDOWN).expect("frontmatter block parses");

    assert_eq!(frontmatter.name, "review-code");
    assert_eq!(
        frontmatter.description,
        "Review Rust code for correctness and style before commit."
    );
    assert_eq!(frontmatter.version.as_deref(), Some("1.2.0"));
    assert_eq!(frontmatter.license.as_deref(), Some("MIT"));
    assert_eq!(
        frontmatter.argument_hint.as_deref(),
        Some("path to the code to review")
    );
    assert_eq!(frontmatter.allowed_tools, vec!["Read", "Grep", "Glob"]);
    assert_eq!(frontmatter.disallowed_tools, vec!["Bash", "Write"]);
    assert_eq!(frontmatter.paths, vec!["src/", "tests/"]);
}

#[test]
fn skill_markdown_without_frontmatter_returns_none() {
    assert!(parse_skill_markdown_frontmatter("# Plain markdown").is_none());
    assert!(parse_skill_markdown_frontmatter("").is_none());
    assert!(parse_skill_markdown_frontmatter("---\n---\n").is_none());
}

#[test]
fn skill_frontmatter_ignores_unknown_keys_and_quotes() {
    let markdown = r#"---
name: "quoted-name"
description: "Quoted description"
custom-key: whatever
---

Body
"#;
    let frontmatter =
        parse_skill_markdown_frontmatter(markdown).expect("quoted frontmatter parses");
    assert_eq!(frontmatter.name, "quoted-name");
    assert_eq!(frontmatter.description, "Quoted description");
}

#[test]
fn skill_descriptor_carries_markdown_and_content_layout() {
    let frontmatter = SkillMarkdownFrontmatter::new("review-code", "review code")
        .with_version("1.0.0")
        .with_license("MIT")
        .with_allowed_tool("Read");

    let layout = SkillContentLayout::with_body("Review the code.")
        .with_reference(
            SkillContentFile::new("references/conventions.md")
                .with_description("local conventions")
                .with_size_hint(2048),
        )
        .with_script(SkillContentFile::new("scripts/check.sh"))
        .with_asset(SkillContentFile::new("assets/report-template.md"));

    let descriptor = AgentSkillDescriptor::new(
        "skill.review",
        "provider.skills",
        "Review Code",
        "review code before commit",
        AgentSkillInvocationMode::ModelInvocable,
    )
    .with_frontmatter(frontmatter)
    .with_content_layout(layout)
    .with_context_budget(4096)
    .with_disallowed_tool("Bash")
    .with_path("src/")
    .with_argument_hint("path to code");

    let frontmatter = descriptor.frontmatter.expect("frontmatter present");
    assert_eq!(frontmatter.license.as_deref(), Some("MIT"));
    assert_eq!(descriptor.context_budget, Some(4096));
    assert_eq!(descriptor.disallowed_tools, vec!["Bash"]);
    assert_eq!(descriptor.argument_hint.as_deref(), Some("path to code"));

    let layout = descriptor.content_layout.expect("layout present");
    assert_eq!(layout.references[0].path, "references/conventions.md");
    assert_eq!(layout.references[0].size_hint, Some(2048));
    assert_eq!(layout.scripts.len(), 1);
    assert_eq!(layout.assets.len(), 1);
}

#[test]
fn skill_visibility_controls_model_invocation() {
    assert!(SkillVisibility::ModelInvocable.allows_model_invocation());
    assert!(!SkillVisibility::UserInvocableOnly.allows_model_invocation());
    assert!(!SkillVisibility::NameOnly.allows_model_invocation());
    assert!(!SkillVisibility::Off.allows_model_invocation());

    let descriptor = AgentSkillDescriptor::new(
        "skill.visibility",
        "provider.skills",
        "Visibility",
        "visibility test",
        AgentSkillInvocationMode::ModelInvocable,
    )
    .with_visibility(SkillVisibility::UserInvocableOnly);
    assert!(!descriptor.allows_model_invocation());

    let off = descriptor.clone().with_visibility(SkillVisibility::Off);
    assert!(!off.allows_tool_invocation("Skill"));
}

#[test]
fn skill_tool_invocation_honors_disallowed_tools() {
    let descriptor = AgentSkillDescriptor::new(
        "skill.guard",
        "provider.skills",
        "Guard",
        "guarded skill",
        AgentSkillInvocationMode::ToolBacked,
    )
    .with_disallowed_tool("bash");

    assert!(descriptor.allows_tool_invocation("Skill"));
    assert!(!descriptor.allows_tool_invocation("bash"));
}

#[test]
fn skill_provider_prepare_and_load_contracts() {
    let provider = StaticSkillProvider;
    assert!(provider.prepare_skill("skill.review").is_ok());

    // Default content loading is capability-missing until a provider
    // implements file-backed skills.
    let error = provider
        .load_skill_content("skill.review", SkillContentLayer::References, "x.md")
        .unwrap_err();
    assert_eq!(error.kind(), KernelErrorKind::CapabilityMissing);
}

struct StaticSkillProvider;

impl AgentSkillProvider for StaticSkillProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.skills",
            "skill",
            "static-skills",
            "0.1.0",
            vec!["skill.discover".to_string(), "skill.invoke".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_skills(&self) -> Vec<AgentSkillDescriptor> {
        vec![AgentSkillDescriptor::new(
            "skill.review",
            "provider.skills",
            "Review Code",
            "review code",
            AgentSkillInvocationMode::ModelInvocable,
        )
        .with_side_effect_level(SideEffectLevel::ReadOnly)]
    }

    fn invoke_skill(&self, request: AgentSkillRequest) -> KernelResult<AgentSkillResult> {
        Ok(AgentSkillResult::succeeded(
            request.skill_request_id,
            request.skill_id,
            "reviewed",
        ))
    }
}
