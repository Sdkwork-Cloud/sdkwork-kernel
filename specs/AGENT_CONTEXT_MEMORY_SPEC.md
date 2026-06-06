# SDKWork Agent Context And Memory Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: context frames, provenance, trust boundaries, memory records,
  retention, deletion/export, redaction, and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.context-memory`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_KNOWLEDGE_PROVIDER_SPI_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_EVENT_TELEMETRY_SPEC.md`

Context and memory are distinct. Context is the bounded information assembled
for a run. Memory is durable or retrievable information stored beyond a single
run.

Knowledge retrieval and RAG are also distinct from both. A knowledge base,
wiki, keyword index, graph store, SQL store, vector store, or external search
service implements `KnowledgeProvider` and returns `KnowledgeDocument` or
`KnowledgeSearchResult` values. A context provider may then select, rank, trim,
and explain the resulting `ContextFrame` values before a model call.

## 1. Context Frame

Required fields:

- `context_frame_id`
- `session_id`
- `task_id`
- `source`
- `content`
- `content_type`
- `trust_level`
- `provenance`
- `redaction_classification`
- `created_at`
- `metadata`

Trust levels:

- `trusted_system`
- `trusted_host`
- `user_supplied`
- `tool_output`
- `retrieved_external`
- `agent_message`
- `unknown_untrusted`

Rules:

- Context frames `MUST` preserve source provenance.
- Untrusted context `MUST` be marked before model invocation.
- Context frames `MUST` carry redaction classification.
- Context frames `MUST` preserve content type, created-at timestamp, and
  namespaced metadata when assembled from knowledge, MCP, tool, host, memory, or
  agent-message sources.
- Context trimming `MUST` not remove trust, provenance, content type, or
  policy-critical metadata.

## 2. Context Provider Operations

Required operations:

- `collect`
- `rank`
- `trim`
- `explain`

Rules:

- Collection `MUST` identify source and trust level.
- Ranking `SHOULD` be deterministic when inputs are identical.
- Trimming `MUST` preserve policy-critical frames or explain removal.
- Explain `MUST` describe why context was included.
- Runtime registries `MUST` support multiple typed context providers in one
  agent runtime. The default provider is the deterministic first registered
  provider; callers that require a specific workspace, memory-backed,
  knowledge-backed, or host-provided context assembly strategy `MUST` select it
  by provider id.
- Context provider diagnostics `MUST` report health per registered provider id
  without collapsing multiple context implementations into one manifest entry.
- Context providers `MUST NOT` replace the knowledge provider SPI. They may
  consume `KnowledgeDocument::to_context_frame(...)` output or other
  provenance-preserving frames, but corpus lookup belongs to
  `AGENT_KNOWLEDGE_PROVIDER_SPI_SPEC.md`.

## 3. Memory Record

Required fields:

- `memory_record_id`
- `scope`
- `owner_context`
- `content`
- `content_type`
- `source`
- `trust_level`
- `retention_policy`
- `redaction_classification`
- `created_at`
- `updated_at`
- `policy_decision_id`
- `metadata`

Scopes:

- `session`
- `user`
- `tenant`
- `organization`
- `agent`
- `application`

Rules:

- Scope `MUST` be explicit.
- Multi-tenant hosts `MUST` include tenant/user isolation metadata.
- Memory writes `MUST` pass policy.
- Memory reads `MUST` pass policy when sensitive or cross-scope.
- Memory records containing personal, tenant-sensitive, secret, or regulated
  data `MUST` declare retention policy.
- Memory records `MUST` preserve content type, source, policy decision id, and
  namespaced metadata for audit, export, and retention enforcement.

## 4. Memory Provider Operations

Required operations:

- `provider_manifest`
- `query`
- `write`
- `delete`
- `export`
- `health`

Rules:

- Memory provider manifests `MUST` declare the exact `memory.*` capabilities
  exposed by the typed provider.
- Query results `MUST` preserve provenance and trust metadata.
- Writes `MUST` record policy decision id.
- Delete/export `MUST` be supported when provider stores personal or regulated
  data.
- Provider errors `MUST` map to kernel error kinds.
- Runtime registries `MUST` support multiple typed memory providers in one
  agent runtime. The default provider is the deterministic first registered
  provider; callers that require a specific session, tenant, external, or
  durable memory store `MUST` select it by provider id.
- Memory provider diagnostics `MUST` report health per registered provider id
  without collapsing multiple memory implementations into one manifest entry.
- Memory providers `MUST NOT` be used as the generic RAG abstraction. A vector
  index can back memory only when it stores agent/user/session memory; domain
  documents and product knowledge belong to `KnowledgeProvider`.

## 5. Redaction And Privacy

Rules:

- Context and memory exports `MUST` enforce redaction classification.
- Secrets `MUST` be stored only through approved secret storage, not general
  memory.
- Memory providers `MUST` support retention enforcement when declared.
- Logs and telemetry `MUST` avoid raw memory payloads unless safe.

## 6. Conformance

Required cases:

- Context frame preserves source provenance.
- Untrusted context is marked.
- Context trimming preserves classification.
- Knowledge-derived context preserves knowledge provenance and classification.
- Memory write without policy fails closed.
- Memory query respects scope.
- Memory delete/export works for personal or regulated data.
- Redaction removes secret memory payload from telemetry.

## 7. Acceptance Checklist

- [ ] Context and memory are distinct.
- [ ] Knowledge/RAG retrieval is distinct from context assembly and durable
      memory.
- [ ] Context frames carry provenance and trust level.
- [ ] Memory records carry scope and retention.
- [ ] Sensitive reads/writes require policy.
- [ ] Delete/export is defined for privacy-sensitive memory.
- [ ] Conformance tests cover provenance, untrusted marking, scope, policy,
      redaction, retention, and export/delete.
