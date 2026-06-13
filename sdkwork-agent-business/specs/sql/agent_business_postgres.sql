-- SDKWork Agent Business PostgreSQL Schema Baseline
-- Component: sdkwork-agent-business
-- Domain: intelligence
-- Version: 0.1.0

CREATE OR REPLACE FUNCTION sdkwork_agent_business_capabilities_json_is_standard(input TEXT)
RETURNS BOOLEAN
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    payload JSONB;
BEGIN
    payload := input::jsonb;
    IF jsonb_typeof(payload) <> 'array' THEN
        RETURN FALSE;
    END IF;

    RETURN NOT EXISTS (
        SELECT 1
        FROM jsonb_array_elements(payload) AS capability_values(value)
        WHERE NOT (
            jsonb_typeof(capability_values.value) = 'string'
            AND char_length(capability_values.value #>> '{}') <= 128
            AND (capability_values.value #>> '{}') ~ '^[a-z0-9_-]+(\.[a-z0-9_-]+)+$'
        )
    )
    AND (
        SELECT COUNT(*)
        FROM jsonb_array_elements(payload) AS capability_values(value)
    ) = (
        SELECT COUNT(DISTINCT capability_values.value #>> '{}')
        FROM jsonb_array_elements(payload) AS capability_values(value)
    );
EXCEPTION WHEN others THEN
    RETURN FALSE;
END;
$$;

CREATE OR REPLACE FUNCTION sdkwork_agent_business_memory_modes_json_is_standard(input TEXT)
RETURNS BOOLEAN
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    payload JSONB;
BEGIN
    payload := input::jsonb;
    IF jsonb_typeof(payload) <> 'array' THEN
        RETURN FALSE;
    END IF;
    IF jsonb_array_length(payload) = 0 THEN
        RETURN FALSE;
    END IF;

    RETURN NOT EXISTS (
        SELECT 1
        FROM jsonb_array_elements_text(payload) AS memory_mode_values(value)
        WHERE memory_mode_values.value NOT IN (
            'keyword',
            'sparse',
            'vector',
            'graph',
            'wiki',
            'rule',
            'hybrid'
        )
    )
    AND (
        SELECT COUNT(*)
        FROM jsonb_array_elements_text(payload) AS memory_mode_values(value)
    ) = (
        SELECT COUNT(DISTINCT memory_mode_values.value)
        FROM jsonb_array_elements_text(payload) AS memory_mode_values(value)
    );
EXCEPTION WHEN others THEN
    RETURN FALSE;
END;
$$;

CREATE OR REPLACE FUNCTION sdkwork_agent_business_knowledge_modes_json_is_standard(input TEXT)
RETURNS BOOLEAN
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    payload JSONB;
BEGIN
    payload := input::jsonb;
    IF jsonb_typeof(payload) <> 'array' THEN
        RETURN FALSE;
    END IF;
    IF jsonb_array_length(payload) = 0 THEN
        RETURN FALSE;
    END IF;

    RETURN NOT EXISTS (
        SELECT 1
        FROM jsonb_array_elements_text(payload) AS knowledge_mode_values(value)
        WHERE knowledge_mode_values.value NOT IN (
            'exact',
            'keyword',
            'full_text',
            'structured',
            'graph',
            'wiki',
            'rule',
            'vector',
            'hybrid',
            'llm_rerank',
            'external'
        )
    )
    AND (
        SELECT COUNT(*)
        FROM jsonb_array_elements_text(payload) AS knowledge_mode_values(value)
    ) = (
        SELECT COUNT(DISTINCT knowledge_mode_values.value)
        FROM jsonb_array_elements_text(payload) AS knowledge_mode_values(value)
    );
EXCEPTION WHEN others THEN
    RETURN FALSE;
END;
$$;

CREATE TABLE IF NOT EXISTS a_agent_business (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(64) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    owner_user_id BIGINT NOT NULL,
    agent_id VARCHAR(128) NOT NULL,
    code VARCHAR(128) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    manifest_json TEXT NOT NULL,
    default_code_task_intent_json TEXT,
    implementation_provider_id VARCHAR(128),
    implementation_kind VARCHAR(64),
    implementation_type VARCHAR(64) NOT NULL DEFAULT 'sdkwork-native',
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    version BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT uk_a_agent_business_tenant_agent_id UNIQUE (tenant_id, agent_id),
    CONSTRAINT uk_a_agent_business_tenant_code UNIQUE (tenant_id, code),
    CONSTRAINT ck_a_agent_business_implementation_kind CHECK (
        implementation_kind IS NULL OR implementation_kind IN (
            'manifest-only',
            'typed-local-provider',
            'process-adapter',
            'protocol-adapter'
        )
    ),
    CONSTRAINT ck_a_agent_business_implementation_type CHECK (
        implementation_type IN (
            'sdkwork-native',
            'rig-rust',
            'openai-agents',
            'langchain',
            'langgraph',
            'crewai',
            'autogen',
            'semantic-kernel',
            'custom'
        )
    ),
    CONSTRAINT ck_a_agent_business_implementation_provider_id_standard CHECK (
        implementation_provider_id IS NULL
        OR implementation_provider_id ~ '^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_business_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_business_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_business_tenant_org_status_updated
    ON a_agent_business (tenant_id, organization_id, status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_a_agent_business_tenant_owner_status
    ON a_agent_business (tenant_id, owner_user_id, status);

CREATE TABLE IF NOT EXISTS a_agent_provider_binding (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(96) NOT NULL,
    tenant_id BIGINT NOT NULL,
    agent_id VARCHAR(128) NOT NULL,
    binding_id VARCHAR(128) NOT NULL,
    provider_id VARCHAR(128) NOT NULL,
    implementation_kind VARCHAR(64) NOT NULL,
    configuration_profile_id VARCHAR(128) NOT NULL,
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    active BOOLEAN NOT NULL DEFAULT FALSE,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_provider_binding_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_provider_binding_tenant_agent_binding UNIQUE (
        tenant_id,
        agent_id,
        binding_id
    ),
    CONSTRAINT ck_a_agent_provider_binding_implementation_kind CHECK (
        implementation_kind IN (
            'manifest-only',
            'typed-local-provider',
            'process-adapter',
            'protocol-adapter'
        )
    ),
    CONSTRAINT ck_a_agent_provider_binding_binding_id_standard CHECK (
        binding_id ~ '^binding\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_provider_binding_provider_id_standard CHECK (
        provider_id ~ '^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_provider_binding_configuration_profile_id_standard CHECK (
        configuration_profile_id ~ '^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_provider_binding_capabilities_standard CHECK (
        sdkwork_agent_business_capabilities_json_is_standard(capabilities_json)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_a_agent_provider_binding_active_default
    ON a_agent_provider_binding (tenant_id, agent_id)
    WHERE active = TRUE;

CREATE INDEX IF NOT EXISTS idx_a_agent_provider_binding_tenant_agent_updated
    ON a_agent_provider_binding (tenant_id, agent_id, active DESC, updated_at DESC, binding_id ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_provider_binding_tenant_provider
    ON a_agent_provider_binding (tenant_id, provider_id);

CREATE TABLE IF NOT EXISTS a_agent_deployment (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(96) NOT NULL,
    tenant_id BIGINT NOT NULL,
    agent_id VARCHAR(128) NOT NULL,
    deployment_id VARCHAR(128) NOT NULL,
    binding_id VARCHAR(128) NOT NULL,
    provider_id_snapshot VARCHAR(128) NOT NULL,
    implementation_kind_snapshot VARCHAR(64) NOT NULL,
    configuration_profile_id_snapshot VARCHAR(128) NOT NULL,
    capabilities_snapshot_json TEXT NOT NULL DEFAULT '[]',
    status SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_deployment_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_deployment_tenant_agent_deployment UNIQUE (
        tenant_id,
        agent_id,
        deployment_id
    ),
    CONSTRAINT ck_a_agent_deployment_implementation_kind CHECK (
        implementation_kind_snapshot IN (
            'manifest-only',
            'typed-local-provider',
            'process-adapter',
            'protocol-adapter'
        )
    ),
    CONSTRAINT ck_a_agent_deployment_deployment_id_standard CHECK (
        deployment_id ~ '^deployment\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_deployment_binding_id_standard CHECK (
        binding_id ~ '^binding\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_deployment_provider_id_snapshot_standard CHECK (
        provider_id_snapshot ~ '^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_deployment_configuration_profile_id_snapshot_standard CHECK (
        configuration_profile_id_snapshot ~ '^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_deployment_capabilities_snapshot_standard CHECK (
        sdkwork_agent_business_capabilities_json_is_standard(capabilities_snapshot_json)
    ),
    CONSTRAINT ck_a_agent_deployment_status CHECK (status IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_deployment_tenant_agent_created
    ON a_agent_deployment (tenant_id, agent_id, created_at DESC, deployment_id ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_deployment_tenant_provider_status
    ON a_agent_deployment (tenant_id, provider_id_snapshot, status);

CREATE TABLE IF NOT EXISTS a_agent_skill_package (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(96) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    owner_user_id BIGINT NOT NULL,
    skill_id VARCHAR(128) NOT NULL,
    code VARCHAR(128) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    invocation_kind VARCHAR(64) NOT NULL,
    package_ref TEXT NOT NULL,
    entrypoint VARCHAR(255) NOT NULL,
    input_schema_json TEXT NOT NULL,
    output_schema_json TEXT NOT NULL,
    capability_ids_json TEXT NOT NULL DEFAULT '[]',
    categories_json TEXT NOT NULL DEFAULT '[]',
    tags_json TEXT NOT NULL DEFAULT '[]',
    security_profile_id VARCHAR(128),
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_skill_package_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_skill_package_tenant_skill UNIQUE (tenant_id, skill_id),
    CONSTRAINT uk_a_agent_skill_package_tenant_code UNIQUE (tenant_id, code),
    CONSTRAINT ck_a_agent_skill_package_skill_id_standard CHECK (
        skill_id ~ '^skill\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_skill_package_invocation_kind CHECK (
        invocation_kind IN (
            'local-workflow',
            'process-adapter',
            'mcp-tool',
            'kernel-provider'
        )
    ),
    CONSTRAINT ck_a_agent_skill_package_capabilities_standard CHECK (
        sdkwork_agent_business_capabilities_json_is_standard(capability_ids_json)
    ),
    CONSTRAINT ck_a_agent_skill_package_security_profile_standard CHECK (
        security_profile_id IS NULL
        OR security_profile_id ~ '^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_skill_package_input_schema_json CHECK (input_schema_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_skill_package_output_schema_json CHECK (output_schema_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_skill_package_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_skill_package_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_skill_package_tenant_org_status_updated
    ON a_agent_skill_package (tenant_id, organization_id, status, updated_at DESC, code ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_skill_package_tenant_visibility_status
    ON a_agent_skill_package (tenant_id, visibility, status);

CREATE TABLE IF NOT EXISTS a_agent_mcp_server (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(96) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    owner_user_id BIGINT NOT NULL,
    mcp_server_id VARCHAR(128) NOT NULL,
    code VARCHAR(128) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    protocol_version VARCHAR(32) NOT NULL,
    transport_kind VARCHAR(32) NOT NULL,
    endpoint_ref VARCHAR(128),
    command_ref VARCHAR(128),
    auth_kind VARCHAR(32) NOT NULL,
    auth_profile_id VARCHAR(128),
    capability_ids_json TEXT NOT NULL DEFAULT '[]',
    tool_count BIGINT NOT NULL DEFAULT 0,
    resource_count BIGINT NOT NULL DEFAULT 0,
    prompt_count BIGINT NOT NULL DEFAULT 0,
    capabilities_json TEXT NOT NULL DEFAULT '{}',
    categories_json TEXT NOT NULL DEFAULT '[]',
    tags_json TEXT NOT NULL DEFAULT '[]',
    security_profile_id VARCHAR(128),
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_mcp_server_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_mcp_server_tenant_server UNIQUE (tenant_id, mcp_server_id),
    CONSTRAINT uk_a_agent_mcp_server_tenant_code UNIQUE (tenant_id, code),
    CONSTRAINT ck_a_agent_mcp_server_id_standard CHECK (
        mcp_server_id ~ '^mcp\.server\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_mcp_server_transport_kind CHECK (
        transport_kind IN ('stdio', 'http', 'sse', 'websocket')
    ),
    CONSTRAINT ck_a_agent_mcp_server_transport_refs CHECK (
        (transport_kind = 'stdio' AND command_ref IS NOT NULL)
        OR (transport_kind IN ('http', 'sse', 'websocket') AND endpoint_ref IS NOT NULL)
    ),
    CONSTRAINT ck_a_agent_mcp_server_endpoint_ref_standard CHECK (
        endpoint_ref IS NULL
        OR endpoint_ref ~ '^endpoint\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_mcp_server_command_ref_standard CHECK (
        command_ref IS NULL
        OR command_ref ~ '^command\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_mcp_server_auth_kind CHECK (
        auth_kind IN ('none', 'oauth2', 'api-key-ref', 'host-secret-ref')
    ),
    CONSTRAINT ck_a_agent_mcp_server_auth_profile_standard CHECK (
        auth_profile_id IS NULL
        OR auth_profile_id ~ '^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_mcp_server_security_profile_standard CHECK (
        security_profile_id IS NULL
        OR security_profile_id ~ '^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_mcp_server_capability_ids_standard CHECK (
        sdkwork_agent_business_capabilities_json_is_standard(capability_ids_json)
    ),
    CONSTRAINT ck_a_agent_mcp_server_counts_non_negative CHECK (
        tool_count >= 0 AND resource_count >= 0 AND prompt_count >= 0
    ),
    CONSTRAINT ck_a_agent_mcp_server_capabilities_json CHECK (capabilities_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_mcp_server_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_mcp_server_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_mcp_server_tenant_org_status_updated
    ON a_agent_mcp_server (tenant_id, organization_id, status, updated_at DESC, code ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_mcp_server_tenant_transport_auth
    ON a_agent_mcp_server (tenant_id, transport_kind, auth_kind);

CREATE TABLE IF NOT EXISTS a_agent_prompt_template (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(96) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    owner_user_id BIGINT NOT NULL,
    prompt_id VARCHAR(128) NOT NULL,
    code VARCHAR(128) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    prompt_kind VARCHAR(32) NOT NULL,
    template_format VARCHAR(32) NOT NULL,
    template_body TEXT NOT NULL,
    variables_schema_json TEXT NOT NULL,
    model_constraints_json TEXT NOT NULL,
    capability_ids_json TEXT NOT NULL DEFAULT '[]',
    categories_json TEXT NOT NULL DEFAULT '[]',
    tags_json TEXT NOT NULL DEFAULT '[]',
    safety_profile_id VARCHAR(128),
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_prompt_template_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_prompt_template_tenant_prompt UNIQUE (tenant_id, prompt_id),
    CONSTRAINT uk_a_agent_prompt_template_tenant_code UNIQUE (tenant_id, code),
    CONSTRAINT ck_a_agent_prompt_template_prompt_id_standard CHECK (
        prompt_id ~ '^prompt\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_prompt_template_prompt_kind CHECK (
        prompt_kind IN ('system', 'developer', 'user', 'workflow', 'tool', 'mcp-prompt')
    ),
    CONSTRAINT ck_a_agent_prompt_template_format CHECK (
        template_format IN ('plain-text', 'handlebars', 'liquid', 'jinja', 'json-schema')
    ),
    CONSTRAINT ck_a_agent_prompt_template_capabilities_standard CHECK (
        sdkwork_agent_business_capabilities_json_is_standard(capability_ids_json)
    ),
    CONSTRAINT ck_a_agent_prompt_template_safety_profile_standard CHECK (
        safety_profile_id IS NULL
        OR safety_profile_id ~ '^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_prompt_template_variables_schema_json CHECK (variables_schema_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_prompt_template_model_constraints_json CHECK (model_constraints_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_prompt_template_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_prompt_template_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_prompt_template_tenant_org_status_updated
    ON a_agent_prompt_template (tenant_id, organization_id, status, updated_at DESC, code ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_prompt_template_tenant_visibility_status
    ON a_agent_prompt_template (tenant_id, visibility, status);

CREATE TABLE IF NOT EXISTS a_agent_knowledge_base (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    owner_user_id BIGINT NOT NULL,
    knowledge_base_id VARCHAR(128) NOT NULL,
    code VARCHAR(128) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    provider_id VARCHAR(128) NOT NULL,
    base_kind VARCHAR(32) NOT NULL,
    retrieval_modes_json TEXT NOT NULL DEFAULT '[]',
    capability_ids_json TEXT NOT NULL DEFAULT '[]',
    configuration_profile_id VARCHAR(128) NOT NULL,
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_knowledge_base_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_knowledge_base_tenant_base UNIQUE (tenant_id, knowledge_base_id),
    CONSTRAINT uk_a_agent_knowledge_base_tenant_code UNIQUE (tenant_id, code),
    CONSTRAINT ck_a_agent_knowledge_base_id_standard CHECK (
        knowledge_base_id ~ '^knowledge\.base\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_base_provider_standard CHECK (
        provider_id ~ '^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_base_kind CHECK (
        base_kind IN (
            'wiki',
            'document-repository',
            'database',
            'api-reference',
            'graph',
            'hybrid',
            'external-provider',
            'file-store'
        )
    ),
    CONSTRAINT ck_a_agent_knowledge_base_retrieval_modes_json CHECK (
        sdkwork_agent_business_knowledge_modes_json_is_standard(retrieval_modes_json)
    ),
    CONSTRAINT ck_a_agent_knowledge_base_capabilities_standard CHECK (
        sdkwork_agent_business_capabilities_json_is_standard(capability_ids_json)
    ),
    CONSTRAINT ck_a_agent_knowledge_base_configuration_profile_standard CHECK (
        configuration_profile_id ~ '^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_base_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_knowledge_base_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_base_tenant_org_status_updated
    ON a_agent_knowledge_base (tenant_id, organization_id, status, updated_at DESC, code ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_base_tenant_provider_kind
    ON a_agent_knowledge_base (tenant_id, provider_id, base_kind);

CREATE TABLE IF NOT EXISTS a_agent_knowledge_source (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    knowledge_source_id VARCHAR(128) NOT NULL,
    knowledge_base_id VARCHAR(128) NOT NULL,
    source_kind VARCHAR(32) NOT NULL,
    source_ref TEXT NOT NULL,
    source_hash VARCHAR(128) NOT NULL,
    sync_policy_json TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    status SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_knowledge_source_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_knowledge_source_tenant_source UNIQUE (tenant_id, knowledge_source_id),
    CONSTRAINT ck_a_agent_knowledge_source_id_standard CHECK (
        knowledge_source_id ~ '^knowledge\.source\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_source_base_id_standard CHECK (
        knowledge_base_id ~ '^knowledge\.base\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_source_kind CHECK (
        source_kind IN (
            'upload',
            'wiki',
            'web',
            'database',
            'api',
            'filesystem',
            'manual',
            'external-provider'
        )
    ),
    CONSTRAINT ck_a_agent_knowledge_source_sync_policy_json CHECK (sync_policy_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_knowledge_source_metadata_json CHECK (metadata_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_knowledge_source_status CHECK (status IN (0, 1, 2, 3, 4))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_source_tenant_base_updated
    ON a_agent_knowledge_source (tenant_id, knowledge_base_id, updated_at DESC, knowledge_source_id ASC);

CREATE TABLE IF NOT EXISTS a_agent_knowledge_document (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    knowledge_document_id VARCHAR(128) NOT NULL,
    knowledge_base_id VARCHAR(128) NOT NULL,
    knowledge_source_id VARCHAR(128),
    document_kind VARCHAR(32) NOT NULL,
    title VARCHAR(512) NOT NULL,
    content_ref TEXT NOT NULL,
    content_hash VARCHAR(128) NOT NULL,
    summary TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    tags_json TEXT NOT NULL DEFAULT '[]',
    categories_json TEXT NOT NULL DEFAULT '[]',
    trust_level SMALLINT NOT NULL DEFAULT 0,
    redaction_classification VARCHAR(64) NOT NULL,
    chunk_count BIGINT NOT NULL DEFAULT 0,
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_knowledge_document_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_knowledge_document_tenant_document UNIQUE (tenant_id, knowledge_document_id),
    CONSTRAINT ck_a_agent_knowledge_document_id_standard CHECK (
        knowledge_document_id ~ '^knowledge\.document\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_document_base_id_standard CHECK (
        knowledge_base_id ~ '^knowledge\.base\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_document_source_id_standard CHECK (
        knowledge_source_id IS NULL
        OR knowledge_source_id ~ '^knowledge\.source\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_document_kind CHECK (
        document_kind IN (
            'wiki-page',
            'wiki-section',
            'article',
            'faq',
            'api-reference',
            'spec',
            'runbook',
            'policy',
            'external-reference',
            'other'
        )
    ),
    CONSTRAINT ck_a_agent_knowledge_document_metadata_json CHECK (metadata_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_knowledge_document_tags_json CHECK (tags_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_knowledge_document_categories_json CHECK (categories_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_knowledge_document_trust_level CHECK (trust_level >= 0 AND trust_level <= 5),
    CONSTRAINT ck_a_agent_knowledge_document_chunk_count CHECK (chunk_count >= 0),
    CONSTRAINT ck_a_agent_knowledge_document_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_knowledge_document_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_document_tenant_base_updated
    ON a_agent_knowledge_document (tenant_id, knowledge_base_id, updated_at DESC, knowledge_document_id ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_document_tenant_source_updated
    ON a_agent_knowledge_document (tenant_id, knowledge_source_id, updated_at DESC)
    WHERE knowledge_source_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS a_agent_knowledge_chunk (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    knowledge_chunk_id VARCHAR(128) NOT NULL,
    knowledge_document_id VARCHAR(128) NOT NULL,
    parent_chunk_id VARCHAR(128),
    chunk_ordinal BIGINT NOT NULL,
    heading VARCHAR(512),
    content_ref TEXT NOT NULL,
    content_hash VARCHAR(128) NOT NULL,
    token_estimate BIGINT NOT NULL,
    summary TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    status SMALLINT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_knowledge_chunk_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_knowledge_chunk_tenant_chunk UNIQUE (tenant_id, knowledge_chunk_id),
    CONSTRAINT uk_a_agent_knowledge_chunk_tenant_document_ordinal UNIQUE (
        tenant_id,
        knowledge_document_id,
        chunk_ordinal
    ),
    CONSTRAINT ck_a_agent_knowledge_chunk_id_standard CHECK (
        knowledge_chunk_id ~ '^knowledge\.chunk\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_chunk_document_id_standard CHECK (
        knowledge_document_id ~ '^knowledge\.document\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_chunk_parent_id_standard CHECK (
        parent_chunk_id IS NULL
        OR parent_chunk_id ~ '^knowledge\.chunk\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_chunk_ordinal CHECK (chunk_ordinal > 0),
    CONSTRAINT ck_a_agent_knowledge_chunk_token_estimate CHECK (token_estimate > 0),
    CONSTRAINT ck_a_agent_knowledge_chunk_metadata_json CHECK (metadata_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_knowledge_chunk_status CHECK (status IN (0, 1, 2, 3, 4))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_chunk_tenant_document_ordinal
    ON a_agent_knowledge_chunk (tenant_id, knowledge_document_id, chunk_ordinal ASC, knowledge_chunk_id ASC);

CREATE TABLE IF NOT EXISTS a_agent_knowledge_index (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    knowledge_index_id VARCHAR(128) NOT NULL,
    knowledge_base_id VARCHAR(128) NOT NULL,
    knowledge_document_id VARCHAR(128),
    knowledge_chunk_id VARCHAR(128),
    index_kind VARCHAR(32) NOT NULL,
    index_provider_id VARCHAR(128) NOT NULL,
    external_ref TEXT NOT NULL,
    embedding_model_id VARCHAR(128),
    vector_dimension BIGINT,
    content_hash VARCHAR(128) NOT NULL,
    indexed_at TIMESTAMP NOT NULL,
    status SMALLINT NOT NULL,
    CONSTRAINT uk_a_agent_knowledge_index_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_knowledge_index_tenant_index UNIQUE (tenant_id, knowledge_index_id),
    CONSTRAINT ck_a_agent_knowledge_index_id_standard CHECK (
        knowledge_index_id ~ '^knowledge\.index\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_index_base_id_standard CHECK (
        knowledge_base_id ~ '^knowledge\.base\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_index_document_id_standard CHECK (
        knowledge_document_id IS NULL
        OR knowledge_document_id ~ '^knowledge\.document\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_index_chunk_id_standard CHECK (
        knowledge_chunk_id IS NULL
        OR knowledge_chunk_id ~ '^knowledge\.chunk\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_index_chunk_requires_document CHECK (
        knowledge_chunk_id IS NULL
        OR knowledge_document_id IS NOT NULL
    ),
    CONSTRAINT ck_a_agent_knowledge_index_kind CHECK (
        index_kind IN (
            'exact',
            'keyword',
            'full_text',
            'structured',
            'graph',
            'wiki',
            'rule',
            'vector',
            'hybrid',
            'llm_rerank',
            'external'
        )
    ),
    CONSTRAINT ck_a_agent_knowledge_index_provider_standard CHECK (
        index_provider_id ~ '^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_index_embedding_model_standard CHECK (
        embedding_model_id IS NULL
        OR embedding_model_id ~ '^model\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_index_vector_contract CHECK (
        index_kind <> 'vector'
        OR (embedding_model_id IS NOT NULL AND vector_dimension IS NOT NULL AND vector_dimension > 0)
    ),
    CONSTRAINT ck_a_agent_knowledge_index_status CHECK (status IN (0, 1, 2, 3, 4))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_index_tenant_document_indexed
    ON a_agent_knowledge_index (tenant_id, knowledge_document_id, indexed_at DESC, knowledge_index_id ASC)
    WHERE knowledge_document_id IS NOT NULL AND status <> 4;

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_index_tenant_base_indexed
    ON a_agent_knowledge_index (tenant_id, knowledge_base_id, indexed_at DESC, knowledge_index_id ASC)
    WHERE status <> 4;

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_index_tenant_kind_provider
    ON a_agent_knowledge_index (tenant_id, index_kind, index_provider_id, status);

CREATE TABLE IF NOT EXISTS a_agent_knowledge_binding (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    knowledge_binding_id VARCHAR(128) NOT NULL,
    knowledge_base_id VARCHAR(128) NOT NULL,
    agent_id VARCHAR(128),
    deployment_id VARCHAR(128),
    scope_kind VARCHAR(32) NOT NULL,
    scope_ref VARCHAR(128) NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    default_binding BOOLEAN NOT NULL DEFAULT FALSE,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_knowledge_binding_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_knowledge_binding_tenant_binding UNIQUE (tenant_id, knowledge_binding_id),
    CONSTRAINT ck_a_agent_knowledge_binding_id_standard CHECK (
        knowledge_binding_id ~ '^knowledge\.binding\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_binding_base_id_standard CHECK (
        knowledge_base_id ~ '^knowledge\.base\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_binding_agent_id_standard CHECK (
        agent_id IS NULL OR agent_id ~ '^agent\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_binding_deployment_id_standard CHECK (
        deployment_id IS NULL OR deployment_id ~ '^deployment\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_binding_scope_kind CHECK (
        scope_kind IN ('agent', 'deployment', 'user', 'session', 'organization', 'tenant')
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_a_agent_knowledge_binding_default_scope
    ON a_agent_knowledge_binding (tenant_id, knowledge_base_id, scope_kind, scope_ref)
    WHERE default_binding = TRUE AND active = TRUE;

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_binding_tenant_base_active
    ON a_agent_knowledge_binding (tenant_id, knowledge_base_id, active DESC, default_binding DESC, updated_at DESC);

CREATE TABLE IF NOT EXISTS a_agent_knowledge_sync_job (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    sync_job_id VARCHAR(128) NOT NULL,
    knowledge_base_id VARCHAR(128) NOT NULL,
    knowledge_source_id VARCHAR(128),
    job_kind VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL,
    input_ref TEXT NOT NULL,
    input_json TEXT NOT NULL,
    output_json TEXT,
    error_json TEXT,
    requested_at TIMESTAMP NOT NULL,
    started_at TIMESTAMP NULL,
    completed_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_knowledge_sync_job_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_knowledge_sync_job_tenant_job UNIQUE (tenant_id, sync_job_id),
    CONSTRAINT ck_a_agent_knowledge_sync_job_id_standard CHECK (
        sync_job_id ~ '^knowledge\.sync\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_sync_job_base_id_standard CHECK (
        knowledge_base_id ~ '^knowledge\.base\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_sync_job_source_id_standard CHECK (
        knowledge_source_id IS NULL
        OR knowledge_source_id ~ '^knowledge\.source\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_knowledge_sync_job_kind CHECK (
        job_kind IN ('import', 'refresh', 'reindex', 'delete')
    ),
    CONSTRAINT ck_a_agent_knowledge_sync_job_status CHECK (
        status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')
    ),
    CONSTRAINT ck_a_agent_knowledge_sync_job_input_json CHECK (input_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_knowledge_sync_job_output_json CHECK (
        output_json IS NULL OR output_json::jsonb IS NOT NULL
    ),
    CONSTRAINT ck_a_agent_knowledge_sync_job_error_json CHECK (
        error_json IS NULL OR error_json::jsonb IS NOT NULL
    )
);

CREATE INDEX IF NOT EXISTS idx_a_agent_knowledge_sync_job_tenant_base_status_requested
    ON a_agent_knowledge_sync_job (tenant_id, knowledge_base_id, status, requested_at DESC);

CREATE TABLE IF NOT EXISTS a_agent_memory_store (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    owner_user_id BIGINT NOT NULL,
    memory_store_id VARCHAR(128) NOT NULL,
    code VARCHAR(128) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    provider_id VARCHAR(128) NOT NULL,
    store_kind VARCHAR(32) NOT NULL,
    retrieval_modes_json TEXT NOT NULL DEFAULT '[]',
    capability_ids_json TEXT NOT NULL DEFAULT '[]',
    configuration_profile_id VARCHAR(128) NOT NULL,
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_memory_store_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_memory_store_tenant_store UNIQUE (tenant_id, memory_store_id),
    CONSTRAINT uk_a_agent_memory_store_tenant_code UNIQUE (tenant_id, code),
    CONSTRAINT ck_a_agent_memory_store_id_standard CHECK (
        memory_store_id ~ '^memory\.store\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_store_provider_id_standard CHECK (
        provider_id ~ '^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_store_kind CHECK (
        store_kind IN (
            'local-postgres',
            'external-provider',
            'vector-store',
            'graph-store',
            'hybrid-store',
            'file-store'
        )
    ),
    CONSTRAINT ck_a_agent_memory_store_retrieval_modes_json CHECK (
        sdkwork_agent_business_memory_modes_json_is_standard(retrieval_modes_json)
    ),
    CONSTRAINT ck_a_agent_memory_store_capabilities_standard CHECK (
        sdkwork_agent_business_capabilities_json_is_standard(capability_ids_json)
    ),
    CONSTRAINT ck_a_agent_memory_store_configuration_profile_standard CHECK (
        configuration_profile_id ~ '^profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_store_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_memory_store_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_store_tenant_org_status_updated
    ON a_agent_memory_store (tenant_id, organization_id, status, updated_at DESC, code ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_store_tenant_provider_kind
    ON a_agent_memory_store (tenant_id, provider_id, store_kind);

CREATE TABLE IF NOT EXISTS a_agent_memory_profile (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    owner_user_id BIGINT NOT NULL,
    memory_profile_id VARCHAR(128) NOT NULL,
    memory_store_id VARCHAR(128) NOT NULL,
    code VARCHAR(128) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    write_policy_json TEXT NOT NULL,
    retrieval_policy_json TEXT NOT NULL,
    compaction_policy_json TEXT NOT NULL,
    retention_policy_json TEXT NOT NULL,
    privacy_policy_json TEXT NOT NULL,
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_memory_profile_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_memory_profile_tenant_profile UNIQUE (tenant_id, memory_profile_id),
    CONSTRAINT uk_a_agent_memory_profile_tenant_code UNIQUE (tenant_id, code),
    CONSTRAINT ck_a_agent_memory_profile_id_standard CHECK (
        memory_profile_id ~ '^memory\.profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_profile_store_id_standard CHECK (
        memory_store_id ~ '^memory\.store\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_profile_write_policy_json CHECK (write_policy_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_memory_profile_retrieval_policy_json CHECK (retrieval_policy_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_memory_profile_compaction_policy_json CHECK (compaction_policy_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_memory_profile_retention_policy_json CHECK (retention_policy_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_memory_profile_privacy_policy_json CHECK (privacy_policy_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_memory_profile_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_memory_profile_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_profile_tenant_store_status
    ON a_agent_memory_profile (tenant_id, memory_store_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS a_agent_memory_binding (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    memory_binding_id VARCHAR(128) NOT NULL,
    memory_profile_id VARCHAR(128) NOT NULL,
    agent_id VARCHAR(128),
    deployment_id VARCHAR(128),
    scope_kind VARCHAR(32) NOT NULL,
    scope_ref VARCHAR(128) NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    default_binding BOOLEAN NOT NULL DEFAULT FALSE,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_memory_binding_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_memory_binding_tenant_binding UNIQUE (tenant_id, memory_binding_id),
    CONSTRAINT ck_a_agent_memory_binding_id_standard CHECK (
        memory_binding_id ~ '^memory\.binding\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_binding_profile_id_standard CHECK (
        memory_profile_id ~ '^memory\.profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_binding_agent_id_standard CHECK (
        agent_id IS NULL OR agent_id ~ '^agent\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_binding_deployment_id_standard CHECK (
        deployment_id IS NULL OR deployment_id ~ '^deployment\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_binding_scope_kind CHECK (
        scope_kind IN ('agent', 'deployment', 'user', 'session', 'organization', 'tenant')
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_a_agent_memory_binding_default_scope
    ON a_agent_memory_binding (tenant_id, memory_profile_id, scope_kind, scope_ref)
    WHERE default_binding = TRUE AND active = TRUE;

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_binding_tenant_agent_active
    ON a_agent_memory_binding (tenant_id, agent_id, active DESC, updated_at DESC);

CREATE TABLE IF NOT EXISTS a_agent_memory_namespace (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    memory_namespace_id VARCHAR(128) NOT NULL,
    agent_id VARCHAR(128),
    user_ref VARCHAR(128),
    session_ref VARCHAR(128),
    thread_ref VARCHAR(128),
    namespace_kind VARCHAR(32) NOT NULL,
    status SMALLINT NOT NULL,
    visibility SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_memory_namespace_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_memory_namespace_tenant_namespace UNIQUE (tenant_id, memory_namespace_id),
    CONSTRAINT ck_a_agent_memory_namespace_id_standard CHECK (
        memory_namespace_id ~ '^memory\.namespace\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_namespace_agent_id_standard CHECK (
        agent_id IS NULL OR agent_id ~ '^agent\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_namespace_kind CHECK (
        namespace_kind IN ('tenant', 'organization', 'agent', 'user', 'session', 'thread', 'task')
    ),
    CONSTRAINT ck_a_agent_memory_namespace_status CHECK (status IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_a_agent_memory_namespace_visibility CHECK (visibility IN (0, 1, 2, 3))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_namespace_tenant_agent_kind
    ON a_agent_memory_namespace (tenant_id, agent_id, namespace_kind, status);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_namespace_tenant_refs
    ON a_agent_memory_namespace (tenant_id, user_ref, session_ref, thread_ref);

CREATE TABLE IF NOT EXISTS a_agent_memory_record (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    memory_id VARCHAR(128) NOT NULL,
    memory_namespace_id VARCHAR(128) NOT NULL,
    agent_id VARCHAR(128),
    memory_kind VARCHAR(32) NOT NULL,
    content_format VARCHAR(64) NOT NULL,
    content_json TEXT NOT NULL,
    summary TEXT,
    salience_score REAL NOT NULL,
    confidence_score REAL NOT NULL,
    freshness_score REAL NOT NULL,
    sensitivity_level SMALLINT NOT NULL DEFAULT 0,
    source_count BIGINT NOT NULL DEFAULT 0,
    effective_at TIMESTAMP NULL,
    expires_at TIMESTAMP NULL,
    last_used_at TIMESTAMP NULL,
    use_count BIGINT NOT NULL DEFAULT 0,
    status SMALLINT NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted_at TIMESTAMP NULL,
    redacted_at TIMESTAMP NULL,
    CONSTRAINT uk_a_agent_memory_record_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_memory_record_tenant_memory UNIQUE (tenant_id, memory_id),
    CONSTRAINT ck_a_agent_memory_record_id_standard CHECK (
        memory_id ~ '^memory\.record\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_record_namespace_id_standard CHECK (
        memory_namespace_id ~ '^memory\.namespace\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_record_agent_id_standard CHECK (
        agent_id IS NULL OR agent_id ~ '^agent\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_record_kind CHECK (
        memory_kind IN (
            'working',
            'episodic',
            'semantic',
            'procedural',
            'preference',
            'summary',
            'task',
            'correction',
            'system'
        )
    ),
    CONSTRAINT ck_a_agent_memory_record_content_json CHECK (content_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_memory_record_scores CHECK (
        salience_score >= 0 AND salience_score <= 1
        AND confidence_score >= 0 AND confidence_score <= 1
        AND freshness_score >= 0 AND freshness_score <= 1
    ),
    CONSTRAINT ck_a_agent_memory_record_sensitivity CHECK (sensitivity_level >= 0 AND sensitivity_level <= 4),
    CONSTRAINT ck_a_agent_memory_record_counts_non_negative CHECK (
        source_count >= 0 AND use_count >= 0
    ),
    CONSTRAINT ck_a_agent_memory_record_status CHECK (status IN (0, 1, 2, 3, 4))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_record_tenant_namespace_updated
    ON a_agent_memory_record (tenant_id, memory_namespace_id, updated_at DESC, memory_id ASC)
    WHERE status <> 4 AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_record_tenant_agent_kind_salience
    ON a_agent_memory_record (tenant_id, agent_id, memory_kind, salience_score DESC, updated_at DESC)
    WHERE status <> 4 AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_record_tenant_expires
    ON a_agent_memory_record (tenant_id, expires_at)
    WHERE expires_at IS NOT NULL AND status <> 4;

CREATE TABLE IF NOT EXISTS a_agent_memory_source (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    memory_source_id VARCHAR(128) NOT NULL,
    memory_id VARCHAR(128) NOT NULL,
    source_kind VARCHAR(32) NOT NULL,
    source_ref TEXT NOT NULL,
    source_hash VARCHAR(128) NOT NULL,
    evidence_json TEXT NOT NULL,
    captured_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_memory_source_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_memory_source_tenant_source UNIQUE (tenant_id, memory_source_id),
    CONSTRAINT ck_a_agent_memory_source_id_standard CHECK (
        memory_source_id ~ '^memory\.source\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_source_memory_id_standard CHECK (
        memory_id ~ '^memory\.record\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_source_kind CHECK (
        source_kind IN (
            'conversation-message',
            'tool-result',
            'document',
            'knowledge-ref',
            'human-feedback',
            'system-rule',
            'business-event'
        )
    ),
    CONSTRAINT ck_a_agent_memory_source_evidence_json CHECK (evidence_json::jsonb IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_source_tenant_memory_captured
    ON a_agent_memory_source (tenant_id, memory_id, captured_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS a_agent_memory_relation (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    memory_relation_id VARCHAR(128) NOT NULL,
    from_memory_id VARCHAR(128) NOT NULL,
    to_memory_id VARCHAR(128) NOT NULL,
    relation_kind VARCHAR(32) NOT NULL,
    weight REAL NOT NULL,
    valid_from TIMESTAMP NULL,
    valid_until TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_memory_relation_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_memory_relation_tenant_relation UNIQUE (tenant_id, memory_relation_id),
    CONSTRAINT ck_a_agent_memory_relation_id_standard CHECK (
        memory_relation_id ~ '^memory\.relation\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_relation_from_id_standard CHECK (
        from_memory_id ~ '^memory\.record\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_relation_to_id_standard CHECK (
        to_memory_id ~ '^memory\.record\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_relation_distinct_endpoints CHECK (from_memory_id <> to_memory_id),
    CONSTRAINT ck_a_agent_memory_relation_kind CHECK (
        relation_kind IN (
            'supports',
            'contradicts',
            'supersedes',
            'duplicates',
            'depends-on',
            'part-of',
            'about-entity'
        )
    ),
    CONSTRAINT ck_a_agent_memory_relation_weight CHECK (weight >= 0 AND weight <= 1)
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_relation_tenant_from_created
    ON a_agent_memory_relation (tenant_id, from_memory_id, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_relation_tenant_to_created
    ON a_agent_memory_relation (tenant_id, to_memory_id, created_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS a_agent_memory_retrieval_index (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    memory_index_id VARCHAR(128) NOT NULL,
    memory_id VARCHAR(128) NOT NULL,
    index_kind VARCHAR(32) NOT NULL,
    index_provider_id VARCHAR(128) NOT NULL,
    external_ref TEXT NOT NULL,
    embedding_model_id VARCHAR(128),
    vector_dimension BIGINT,
    content_hash VARCHAR(128) NOT NULL,
    indexed_at TIMESTAMP NOT NULL,
    status SMALLINT NOT NULL,
    CONSTRAINT uk_a_agent_memory_retrieval_index_uuid UNIQUE (uuid),
    CONSTRAINT uk_a_agent_memory_retrieval_index_tenant_index UNIQUE (tenant_id, memory_index_id),
    CONSTRAINT ck_a_agent_memory_retrieval_index_id_standard CHECK (
        memory_index_id ~ '^memory\.index\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_retrieval_index_memory_id_standard CHECK (
        memory_id ~ '^memory\.record\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_retrieval_index_kind CHECK (
        index_kind IN ('keyword', 'sparse', 'vector', 'graph', 'wiki', 'rule', 'hybrid')
    ),
    CONSTRAINT ck_a_agent_memory_retrieval_index_provider_standard CHECK (
        index_provider_id ~ '^provider\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_retrieval_index_embedding_model_standard CHECK (
        embedding_model_id IS NULL
        OR embedding_model_id ~ '^model\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_retrieval_index_vector_contract CHECK (
        index_kind <> 'vector'
        OR (embedding_model_id IS NOT NULL AND vector_dimension IS NOT NULL AND vector_dimension > 0)
    ),
    CONSTRAINT ck_a_agent_memory_retrieval_index_status CHECK (status IN (0, 1, 2, 3, 4))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_retrieval_index_tenant_memory_indexed
    ON a_agent_memory_retrieval_index (tenant_id, memory_id, indexed_at DESC, memory_index_id ASC);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_retrieval_index_tenant_kind_provider
    ON a_agent_memory_retrieval_index (tenant_id, index_kind, index_provider_id, status);

CREATE TABLE IF NOT EXISTS a_agent_memory_access_event (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    memory_id VARCHAR(128) NOT NULL,
    memory_namespace_id VARCHAR(128) NOT NULL,
    agent_id VARCHAR(128),
    access_kind VARCHAR(32) NOT NULL,
    subject_ref VARCHAR(128) NOT NULL,
    request_id VARCHAR(128),
    trace_id VARCHAR(128),
    created_at TIMESTAMP NOT NULL,
    CONSTRAINT uk_a_agent_memory_access_event_uuid UNIQUE (uuid),
    CONSTRAINT ck_a_agent_memory_access_event_memory_id_standard CHECK (
        memory_id ~ '^memory\.record\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_access_event_namespace_id_standard CHECK (
        memory_namespace_id ~ '^memory\.namespace\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_access_event_agent_id_standard CHECK (
        agent_id IS NULL OR agent_id ~ '^agent\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_access_event_kind CHECK (
        access_kind IN ('created', 'retrieved', 'used', 'updated', 'redacted', 'deleted', 'compacted')
    )
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_access_event_tenant_memory_created
    ON a_agent_memory_access_event (tenant_id, memory_id, created_at DESC);

CREATE TABLE IF NOT EXISTS a_agent_memory_compaction_job (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(128) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    memory_namespace_id VARCHAR(128) NOT NULL,
    memory_profile_id VARCHAR(128) NOT NULL,
    compaction_kind VARCHAR(32) NOT NULL,
    input_query_json TEXT NOT NULL,
    output_memory_id VARCHAR(128),
    status SMALLINT NOT NULL,
    requested_at TIMESTAMP NOT NULL,
    started_at TIMESTAMP NULL,
    finished_at TIMESTAMP NULL,
    error_json TEXT,
    CONSTRAINT uk_a_agent_memory_compaction_job_uuid UNIQUE (uuid),
    CONSTRAINT ck_a_agent_memory_compaction_job_namespace_id_standard CHECK (
        memory_namespace_id ~ '^memory\.namespace\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_compaction_job_profile_id_standard CHECK (
        memory_profile_id ~ '^memory\.profile\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_compaction_job_output_memory_id_standard CHECK (
        output_memory_id IS NULL
        OR output_memory_id ~ '^memory\.record\.[a-z0-9_-]+(\.[a-z0-9_-]+)*$'
    ),
    CONSTRAINT ck_a_agent_memory_compaction_job_kind CHECK (
        compaction_kind IN ('summarize', 'deduplicate', 'merge', 'expire', 'redact')
    ),
    CONSTRAINT ck_a_agent_memory_compaction_job_input_query_json CHECK (input_query_json::jsonb IS NOT NULL),
    CONSTRAINT ck_a_agent_memory_compaction_job_error_json CHECK (
        error_json IS NULL OR error_json::jsonb IS NOT NULL
    ),
    CONSTRAINT ck_a_agent_memory_compaction_job_status CHECK (status IN (0, 1, 2, 3, 4))
);

CREATE INDEX IF NOT EXISTS idx_a_agent_memory_compaction_job_tenant_namespace_status
    ON a_agent_memory_compaction_job (tenant_id, memory_namespace_id, status, requested_at DESC);

CREATE TABLE IF NOT EXISTS a_agent_business_audit_event (
    id BIGINT NOT NULL PRIMARY KEY,
    uuid VARCHAR(64) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    agent_business_id BIGINT NOT NULL,
    agent_id VARCHAR(128) NOT NULL,
    action VARCHAR(64) NOT NULL,
    subject_id VARCHAR(128) NOT NULL,
    subject_tenant_id VARCHAR(128) NOT NULL,
    request_id VARCHAR(128),
    trace_id VARCHAR(128),
    payload_json TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    CONSTRAINT ck_a_agent_business_audit_action CHECK (
        action IN (
            'created',
            'updated',
            'deleted',
            'restored',
            'status_changed',
            'started',
            'completed',
            'failed',
            'cancelled',
            'provider_binding_changed',
            'deployment_created',
            'skill_created',
            'skill_updated',
            'skill_deleted',
            'skill_restored',
            'mcp_created',
            'mcp_updated',
            'mcp_deleted',
            'mcp_restored',
            'prompt_created',
            'prompt_updated',
            'prompt_deleted',
            'prompt_restored',
            'knowledge_base_created',
            'knowledge_base_updated',
            'knowledge_base_deleted',
            'knowledge_base_restored',
            'knowledge_source_created',
            'knowledge_source_updated',
            'knowledge_source_deleted',
            'knowledge_source_restored',
            'knowledge_document_created',
            'knowledge_document_updated',
            'knowledge_document_deleted',
            'knowledge_document_restored',
            'knowledge_chunk_created',
            'knowledge_index_upserted',
            'knowledge_binding_created',
            'knowledge_sync_job_created',
            'knowledge_sync_job_started',
            'knowledge_sync_job_completed',
            'knowledge_sync_job_failed',
            'knowledge_sync_job_cancelled',
            'memory_store_created',
            'memory_store_updated',
            'memory_store_deleted',
            'memory_store_restored',
            'memory_profile_created',
            'memory_binding_created',
            'memory_namespace_created',
            'memory_record_created',
            'memory_record_deleted',
            'memory_record_restored',
            'memory_source_created',
            'memory_relation_created',
            'memory_retrieval_index_upserted',
            'runtime_executed',
            'memory_profile_updated',
            'memory_profile_deleted',
            'memory_profile_restored',
            'memory_binding_updated',
            'memory_binding_deleted',
            'memory_binding_restored',
            'memory_namespace_updated',
            'memory_namespace_deleted',
            'memory_namespace_restored',
            'memory_source_deleted',
            'memory_source_restored',
            'memory_relation_deleted',
            'memory_relation_restored'
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_a_agent_business_audit_tenant_agent_created
    ON a_agent_business_audit_event (tenant_id, agent_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_a_agent_business_audit_tenant_action_created
    ON a_agent_business_audit_event (tenant_id, action, created_at DESC);
