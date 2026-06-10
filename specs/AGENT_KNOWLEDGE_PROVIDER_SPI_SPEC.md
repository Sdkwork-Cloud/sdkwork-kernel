# SDKWork Agent Knowledge Provider SPI Specification

- Version: 0.1.0
- Status: standard candidate
- Scope: provider-neutral knowledge retrieval, document read/list contracts,
  RAG retrieval boundaries, provenance, redaction, policy, runtime registration,
  and conformance
- Domain: `intelligence`
- Capability: `agent-kernel.knowledge-provider`
- Related:
  - `AGENT_KERNEL_SPEC.md`
  - `AGENT_CONTEXT_MEMORY_SPEC.md`
  - `AGENT_SECURITY_POLICY_SPEC.md`
  - `AGENT_RUNTIME_SPEC.md`

Knowledge retrieval is a separate SPI from memory and context assembly.
Retrieval-augmented generation is modeled as:

1. `KnowledgeProvider` searches or reads domain knowledge.
2. A context assembly layer converts selected knowledge into `ContextFrame`
   values with provenance, trust, and redaction metadata.
3. A model provider generates with that bounded context.

This standard does not require vector search. Vector retrieval, keyword search,
full-text search, graph traversal, SQL/structured lookup, wiki-section lookup,
manual curation, hybrid retrieval, and LLM reranking are implementation details
behind the same provider family.

## 1. Provider Family

Knowledge providers use:

```yaml
provider_family: knowledge
```

Standard capabilities:

- `knowledge.search`
- `knowledge.read`
- `knowledge.list`

Rules:

- Providers `MUST` declare exact `knowledge.*` capabilities in
  `ProviderManifest`.
- Runtime builders `MUST` support manifest-only registration and typed local
  registration for knowledge providers.
- Multiple typed knowledge providers `MUST` be selectable by provider id.
- The deterministic default provider is the first typed knowledge provider
  registered in a runtime.
- Knowledge providers `MUST NOT` be modeled as `memory` providers unless they
  store durable agent/user/tenant state.
- Knowledge providers `MUST NOT` require a vector store. Vector-backed RAG is
  one retrieval method, not the SPI shape.

## 2. Retrieval Methods

Standard retrieval methods:

- `exact`
- `keyword`
- `full_text`
- `structured`
- `graph`
- `vector`
- `hybrid`
- `llm_rerank`
- `external`

Rules:

- Requests MAY specify one or more preferred methods.
- Providers MAY choose a supported method when the request does not specify
  one.
- Search results `MUST` report the method that produced or selected the result.
- Hybrid providers `SHOULD` expose enough metadata for diagnostics to explain
  which method contributed to the final result.

## 3. Search Request

Required fields:

- `query`
- `top_k`
- `methods`
- `filters`
- `include_external`

Optional fields:

- `session_id`
- `task_id`
- `run_id`
- `step_id`
- `tenant_id`
- `namespace`
- `policy_decision_id`
- `trace_context`
- `timeout_ms`
- `metadata`

Rules:

- `query` is provider-neutral text or query intent. Provider-specific query
  syntax `MUST` be documented as a namespaced extension.
- Search requests `MUST` carry runtime correlation context where available,
  including session, task, run, step, trace context, policy decision id, timeout,
  and namespaced metadata.
- `namespace` scopes retrieval to a logical corpus such as a wiki, tenant
  knowledge base, product manual, or API reference set.
- `tenant_id` `MUST` be honored by multi-tenant providers.
- Filters `MUST` use stable string keys and values unless a future schema adds
  typed filter values.
- `include_external=false` `MUST` exclude documents classified as external
  references unless policy explicitly allows them.

## 4. Search Result

Required fields:

- `document_id`
- `document_kind`
- `title`
- `retrieval_method`
- `trust_level`
- `redaction_classification`

Optional fields:

- `snippet`
- `score`
- `source_uri`
- `metadata`

Rules:

- Result scores are provider-local and `MUST NOT` be compared across providers
  unless a reranker explicitly normalizes them.
- Results `MUST` preserve source URI or another stable provenance reference
  when available.
- Retrieved external results `MUST` use `retrieved_external` or a stricter trust
  level unless a trusted host policy classifies them differently.
- Search results containing sensitive content `MUST` carry the correct
  redaction classification before model invocation.

## 5. Knowledge Document

Standard document kinds:

- `wiki_page`
- `wiki_section`
- `article`
- `faq`
- `api_reference`
- `spec`
- `runbook`
- `policy`
- `external_reference`
- `other`

Required fields:

- `document_id`
- `kind`
- `title`
- `content`
- `content_type`
- `trust_level`
- `redaction_classification`

Optional fields:

- `namespace`
- `source_uri`
- `tags`
- `retrieval_methods`
- `metadata`

Rules:

- `read(document_id)` `MUST` return the stable document represented by a search
  result, or fail with a kernel error.
- `list(filter)` `MUST` apply kind, namespace, tag, and external-reference
  filters consistently.
- Read and list operations `MUST` treat soft-deleted or otherwise unavailable
  documents as not found.
- A document can be converted to a `ContextFrame`; conversion `MUST` preserve
  content, content type, provenance, trust level, redaction classification, and
  namespaced metadata.

## 6. RAG Boundary

RAG implementations SHOULD be composed as:

```text
KnowledgeProvider -> context selection/assembly -> ModelProvider
```

Examples:

- A vector database-backed Rig adapter maps vector hits to `knowledge.search`
  results with `retrieval_method: vector`.
- A wiki or llm-wiki implementation maps page and section lookup to
  `knowledge.search` results with `retrieval_method: keyword`, `graph`,
  `structured`, or `hybrid`.
- A SQL or graph knowledge base maps structured queries to
  `retrieval_method: structured` or `graph`.
- An LLM reranker can be a provider-internal step and report
  `retrieval_method: llm_rerank` when it selects the final result.

Rules:

- Knowledge retrieval `MUST NOT` imply model generation.
- Model generation `MUST NOT` directly depend on provider-specific retrieval
  objects such as vector-store hits.
- Context assembly `MUST` decide which retrieved documents enter the prompt and
  must preserve policy-critical metadata.
- Retrieval indexes must preserve scope integrity: document-scoped results must
  belong to the requested knowledge base, chunk-scoped results must also
  reference the owning document, and soft-deleted indexes/documents/chunks
  `MUST NOT` be returned.
- Existing user knowledge bases SHOULD implement `KnowledgeProvider` directly
  or through a thin adapter. Application code passes the provider or its
  provider id to runtime composition rather than passing a Rig-specific vector
  index.

## 7. Policy And Security

Rules:

- `knowledge.search`, `knowledge.read`, and `knowledge.list` are read-only
  capabilities, but policy may still be required for tenant-sensitive,
  personal, regulated, or external corpora.
- Each read-only knowledge capability maps to the same-named policy category:
  `knowledge.search`, `knowledge.read`, and `knowledge.list`.
- Providers `MUST` fail closed when tenant isolation, authorization, or
  provenance cannot be established.
- Raw secrets `MUST NOT` be stored in knowledge documents.
- External documents `MUST` be marked as retrieved external or untrusted unless
  a host policy assigns a narrower trust level.
- Telemetry `MUST NOT` log raw sensitive document content by default.

## 8. Runtime Diagnostics

Runtime diagnostics `MUST` report:

- Manifest-only knowledge providers.
- Typed knowledge providers.
- Provider health by provider id.
- Declared `knowledge.*` capabilities.
- Missing required `knowledge.*` capabilities.

## 9. Conformance

Required cases:

- Typed knowledge provider registers and appears in capability manifest.
- Manifest-only knowledge provider negotiates capabilities but direct SPI
  access returns `provider_unavailable`.
- Runtime can register multiple knowledge providers and select by provider id.
- Search request supports non-vector retrieval methods.
- Search results report retrieval method, document kind, trust, and redaction.
- Document read/list preserve namespace, tags, provenance, trust, and redaction.
- Document-to-context conversion preserves content, provenance, trust, and
  redaction.

## 10. Acceptance Checklist

- [ ] Knowledge is distinct from memory and context assembly.
- [ ] RAG is provider-neutral and not vector-store-specific.
- [ ] `provider_family: knowledge` is available in schemas, manifests,
      runtime diagnostics, and agent definitions.
- [ ] `knowledge.search`, `knowledge.read`, and `knowledge.list` have runtime
      capability metadata.
- [ ] External frameworks such as Rig are wrapped by plugin providers, not
      imported by kernel-core.
