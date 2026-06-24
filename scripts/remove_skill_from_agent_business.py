#!/usr/bin/env python3
"""Remove skill package ownership from sdkwork-agent-business (moved to sdkwork-skills)."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "sdkwork-agent-business"


def remove_block(text: str, start: str, end: str, *, include_start: bool = True) -> str:
    i = text.find(start)
    if i < 0:
        return text
    j = text.find(end, i + len(start))
    if j < 0:
        raise ValueError(f"end marker not found after {start!r}")
    if include_start:
        return text[:i] + text[j:]
    return text[:i] + end + text[j + len(end) :]


def remove_regex_block(text: str, pattern: str, flags: int = re.DOTALL) -> str:
    return re.sub(pattern, "", text, count=1, flags=flags)


def patch_ports() -> None:
    path = ROOT / "src/ports.rs"
    text = path.read_text(encoding="utf-8")
    text = text.replace(
        "    AgentPromptTemplateRecord, AgentProviderBindingRecord, AgentSkillPackageRecord,\n",
        "    AgentPromptTemplateRecord, AgentProviderBindingRecord,\n",
    )
    text = remove_regex_block(
        text,
        r"\n    fn insert_skill_package\(.*?\n    fn insert_mcp_server\(",
        flags=re.DOTALL,
    )
    text = text.replace(
        "\n    fn insert_mcp_server(",
        "\n    fn insert_mcp_server(",
    )
    path.write_text(text, encoding="utf-8")
    print("ports.rs")


def patch_infrastructure() -> None:
    path = ROOT / "src/infrastructure.rs"
    text = path.read_text(encoding="utf-8")
    text = text.replace("AgentSkillPackageRecord, ", "")
    text = text.replace("\n    skill_packages: Vec<AgentSkillPackageRecord>,", "")
    text = text.replace("\n            skill_packages: Vec::new(),", "")
    text = remove_regex_block(
        text,
        r"\n    fn insert_skill_package\(.*?\n    fn insert_mcp_server\(",
        flags=re.DOTALL,
    )
    path.write_text(text, encoding="utf-8")
    print("infrastructure.rs")


def patch_domain() -> None:
    path = ROOT / "src/domain.rs"
    text = path.read_text(encoding="utf-8")
    text = remove_regex_block(
        text,
        r"\n#\[derive\(Debug, Clone, Copy, PartialEq, Eq\)\]\npub enum AgentSkillInvocationKind \{.*?\n\}\n\nimpl AgentSkillInvocationKind \{.*?\n\}\n\n",
        flags=re.DOTALL,
    )
    text = text.replace("impl_domain_from_str!(AgentSkillInvocationKind);\n", "")
    text = text.replace("impl_domain_from_str_compat!(AgentSkillInvocationKind);\n", "")
    text = remove_regex_block(
        text,
        r"\n#\[derive\(Debug, Clone, PartialEq, Eq\)\]\npub struct AgentSkillPackageRecord \{.*?\n\}\n\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\nimpl AgentSkillPackageRecord \{.*?\n\}\n\nimpl AgentMcpServerRecord",
        flags=re.DOTALL,
    )
    text = text.replace(
        "\nimpl AgentMcpServerRecord",
        "\nimpl AgentMcpServerRecord",
    )
    path.write_text(text, encoding="utf-8")
    print("domain.rs")


def patch_lib() -> None:
    path = ROOT / "src/lib.rs"
    text = path.read_text(encoding="utf-8")
    text = text.replace(
        "    AgentSkillPackageCreateCommand, AgentSkillPackageUpdateCommand, ", ""
    )
    text = text.replace(
        "    AgentRuntimeExecutionStatus, AgentSkillInvocationKind, AgentSkillPackageRecord,\n",
        "    AgentRuntimeExecutionStatus,\n",
    )
    text = text.replace(
        "    AgentMemoryStoreRow, AgentPromptTemplateRow, AgentProviderBindingRow, AgentSkillPackageRow,\n",
        "    AgentMemoryStoreRow, AgentPromptTemplateRow, AgentProviderBindingRow,\n",
    )
    path.write_text(encoding="utf-8", data=text)
    print("lib.rs")


def patch_persistence() -> None:
    path = ROOT / "src/persistence.rs"
    text = path.read_text(encoding="utf-8")
    text = text.replace(
        "    AgentSkillInvocationKind, AgentSkillPackageRecord, AgentVisibility,\n",
        "    AgentVisibility,\n",
    )
    text = remove_regex_block(
        text,
        r"pub const SQL_INSERT_AGENT_SKILL_PACKAGE: &str =.*?pub const SQL_LIST_AGENT_SKILL_PACKAGES: &str =.*?\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\n#\[derive\(Debug, Clone, PartialEq, Eq\)\]\npub struct AgentSkillPackageRow \{.*?\n\}\n\nimpl AgentSkillPackageRow \{.*?\n\}\n\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\nfn parse_skill_invocation_kind\(input: &str\) -> KernelResult<AgentSkillInvocationKind> \{.*?\n\}\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\nfn validate_skill_package_storage_contract\(record: &AgentSkillPackageRecord\) -> KernelResult<\(\)> \{.*?\n\}\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\n    fn insert_skill_package_row\(.*?\n    fn insert_mcp_server_row\(",
        flags=re.DOTALL,
    )
    text = text.replace(
        "\n    fn insert_mcp_server_row(",
        "\n    fn insert_mcp_server_row(",
    )
    text = remove_regex_block(
        text,
        r"\n    fn insert_skill_package\(&mut self, record: AgentSkillPackageRecord\) -> KernelResult<\(\)> \{.*?\n    fn insert_mcp_server\(",
        flags=re.DOTALL,
    )
    text = text.replace(
        "\n    fn insert_mcp_server(",
        "\n    fn insert_mcp_server(",
    )
    text = remove_regex_block(
        text,
        r"\n    fn insert_skill_package_row\(&mut self, row: AgentSkillPackageRow\) -> KernelResult<\(\)> \{.*?\n    fn insert_mcp_server_row\(",
        flags=re.DOTALL,
    )
    text = text.replace(
        "\n    fn insert_mcp_server_row(",
        "\n    fn insert_mcp_server_row(",
    )
    text = remove_regex_block(
        text,
        r"\nfn build_agent_skill_package_uuid\(tenant_id: u64, skill_id: &str\) -> String \{.*?\n\}\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\n#\[cfg\(feature = \"postgres-sync\"\)\]\nfn pg_row_to_agent_skill_package_row\(row: PgRow\) -> KernelResult<AgentSkillPackageRow> \{.*?\n\}\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\n    fn sample_skill_package_record\(\) -> AgentSkillPackageRecord \{.*?\n    \}\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\n    #\[test\]\n    fn skill_package_row_roundtrip_preserves_marketplace_contract\(\) \{.*?\n    \}\n",
        flags=re.DOTALL,
    )
    # marketplace_rows_reject_non_standard_ids_from_storage: remove skill block only
    text = text.replace(
        """        let mut skill = AgentSkillPackageRow::from_record(&sample_skill_package_record())
            .expect("skill row should build");
        skill.skill_id = "agent.skill.bad".to_string();
        let error = skill
            .into_record()
            .expect_err("invalid skill id should fail");
        assert_validation_contains(error, "skillId");

        """,
        "",
    )
    # SQL contract tests
    text = re.sub(
        r"\s*assert!\(SQL_INSERT_AGENT_SKILL_PACKAGE\.contains.*?\)\);\n",
        "",
        text,
        flags=re.DOTALL,
    )
    text = re.sub(
        r'\s*"CREATE TABLE IF NOT EXISTS a_agent_skill_package",\n',
        "",
        text,
    )
    for fragment in [
        '"ck_a_agent_skill_package_skill_id_standard",\n',
        '"skill_id ~ \'^skill\\\\.[a-z0-9_-]+(\\\\.[a-z0-9_-]+)*$\',"\n',
        '"ck_a_agent_skill_package_invocation_kind",\n',
        '"ck_a_agent_skill_package_capabilities_standard",\n',
        '"skill_created",\n',
    ]:
        text = text.replace(fragment, "")
    path.write_text(text, encoding="utf-8")
    print("persistence.rs")


def patch_tests() -> None:
    path = ROOT / "tests/agent_marketplace_contracts.rs"
    text = path.read_text(encoding="utf-8")
    text = text.replace(
        "    AgentPromptTemplateUpdateCommand, AgentSkillInvocationKind, AgentSkillPackageCreateCommand,\n"
        "    AgentSkillPackageUpdateCommand, AgentVisibility, AllowAllPolicyProvider,\n",
        "    AgentPromptTemplateUpdateCommand, AgentVisibility, AllowAllPolicyProvider,\n",
    )
    text = remove_regex_block(
        text,
        r"\nfn skill_create_command\(skill_id: &str, code: &str\) -> AgentSkillPackageCreateCommand \{.*?\n\}\n",
        flags=re.DOTALL,
    )
    text = remove_regex_block(
        text,
        r"\n#\[test\]\nfn skill_package_marketplace_crud_enforces_standard_metadata\(\) \{.*?\n\}\n",
        flags=re.DOTALL,
    )
    # marketplace_records_reject_invalid_ids: remove skill id test, keep rest
    text = re.sub(
        r"\n    let invalid_skill_id = service\n        \.create_skill_package\(skill_create_command\(\"agent\.skill\.bad\", \"bad-skill\"\)\)\n        \.expect_err\(\"skill id must use skill prefix\"\);\n    assert_eq!\(invalid_skill_id\.kind\(\), KernelErrorKind::ValidationError\);\n",
        "",
        text,
    )
    path.write_text(text, encoding="utf-8")
    print("agent_marketplace_contracts.rs")

    id_path = ROOT / "tests/agent_id_contracts.rs"
    id_text = id_path.read_text(encoding="utf-8")
    if "skill" in id_text.lower():
        id_text = re.sub(r".*skill.*\n", "", id_text, flags=re.IGNORECASE)
        id_path.write_text(id_text, encoding="utf-8")
    print("agent_id_contracts.rs")


def patch_db_spec() -> None:
    path = ROOT / "specs/AGENT_BUSINESS_DATABASE_SPEC.md"
    if not path.exists():
        return
    text = path.read_text(encoding="utf-8")
    text = re.sub(
        r"## Table: `a_agent_skill_package`.*?(?=## Table:|## |$)",
        "",
        text,
        flags=re.DOTALL,
    )
    text = text.replace("a_agent_skill_package", "*(migrated to sdkwork-skills `ai_agent_skill_package`)*")
    path.write_text(text, encoding="utf-8")
    print("AGENT_BUSINESS_DATABASE_SPEC.md")


def main() -> None:
    patch_ports()
    patch_infrastructure()
    patch_domain()
    patch_lib()
    patch_persistence()
    patch_tests()
    patch_db_spec()
    print("done")


if __name__ == "__main__":
    main()
