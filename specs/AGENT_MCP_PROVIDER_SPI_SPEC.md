# SDKWork Agent MCP Provider SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: MCP server descriptors, tool/resource/prompt exposure, invocation,
  health, policy, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.mcp-provider`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_RUNTIME_SPEC.md`
  - `AGENT_TOOL_PROVIDER_SPI_SPEC.md`
  - `AGENT_PROTOCOL_ADAPTER_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`

The MCP Provider SPI makes Model Context Protocol integration a first-class
kernel extension point without making MCP the internal kernel object model. MCP
servers may expose tools, resources, and prompts. The agent kernel maps those
surfaces into typed SDKWork objects and keeps transport, authentication, and
server lifecycle details behind provider boundaries.

## 1. Provider Family

MCP providers use `provider_family: mcp`.

Standard capabilities:

- `mcp.tools`
- `mcp.resources`
- `mcp.prompts`

Rules:

- MCP tools `MUST` map to `ToolDescriptor`, `ToolCall`, and `ToolResult`.
- MCP resources `MUST` remain resource descriptors and content reads unless a
  provider explicitly exposes a resource as a tool.
- MCP prompts `MUST` remain prompt descriptors and prompt messages unless a
  provider explicitly exposes a prompt as a tool.
- MCP transport details `MUST` stay in provider metadata and must not leak into
  the kernel object model.
- MCP server health `MUST` be observable through provider health.

## 2. Required SPI Objects

Required objects:

- `McpServerDescriptor`
- `McpResourceDescriptor`
- `McpResourceContent`
- `McpPromptDescriptor`
- `McpPromptMessage`
- `McpProvider`

Required operations:

- `provider_manifest`
- `health`
- `list_servers`
- `list_tools`
- `invoke_tool`
- `list_resources`
- `read_resource`
- `list_prompts`
- `get_prompt`

Rules:

- Unsupported MCP surfaces `MUST` return `capability_missing`.
- Tool invocation through MCP `MUST` pass the same policy gates as other tool
  providers.
- Resource reads that may expose private, proprietary, or user data `MUST`
  preserve trust and redaction metadata in downstream context.
- Resource content `MUST` carry trust level, redaction classification, optional
  namespaced metadata, and a deterministic mapping to `ContextFrame` so MCP can
  participate in RAG/context pipelines without becoming the kernel object model.
- Prompt loading `MUST NOT` bypass model, tool, or host policy.
- Prompt messages `MUST` carry trust level, redaction classification, optional
  namespaced metadata, and deterministic mapping to `ContextFrame` values before
  they are used as model context.

## 3. Runtime Registration

Runtime builders `MUST` provide manifest-only and typed registration paths for
MCP providers.

Rules:

- Manifest-only MCP providers are valid for negotiation and introspection.
- Direct local SPI execution against a manifest-only MCP provider `MUST` return
  `provider_unavailable`.
- Runtime registries `MUST` allow multiple typed MCP providers to be registered
  in a single agent runtime.
- The first typed MCP provider is the deterministic default.
- Callers that need a specific MCP implementation `MUST` resolve it by provider
  id.
- Typed MCP providers `MUST` appear in runtime diagnostics as typed registered
  providers.
- The runtime capability manifest `MUST` preserve `provider_family: mcp`,
  provider id, version, and declared `mcp.*` capabilities.

## 4. Conformance

Required cases:

- Typed MCP provider registers and appears in capability manifest.
- Multiple typed MCP providers can coexist and be selected by provider id.
- Manifest-only MCP provider negotiates capabilities but direct execution
  returns `provider_unavailable`.
- Tools map to SDKWork tool descriptors and results.
- Resources map to descriptors and resource content without being forced into
  tools.
- Prompts map to prompt descriptors and messages without being forced into
  tools.
- Provider health appears in runtime diagnostics.
