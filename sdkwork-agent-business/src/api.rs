#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiOperation {
    pub method: &'static str,
    pub path: &'static str,
    pub tag: &'static str,
    pub operation_id: &'static str,
}

pub const AGENT_APP_API_PREFIX: &str = "/app/v3/api";
pub const AGENT_BACKEND_API_PREFIX: &str = "/backend/v3/api";

pub const AGENT_APP_API_OPERATIONS: &[ApiOperation] = &[
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/app/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/app/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/restore",
        tag: "ai",
        operation_id: "agents.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.create",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
        tag: "ai",
        operation_id: "agents.providerBindings.activate",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.create",
    },
];

pub const AGENT_BACKEND_API_OPERATIONS: &[ApiOperation] = &[
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/backend/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.update",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/status",
        tag: "ai",
        operation_id: "agents.status.update",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/restore",
        tag: "ai",
        operation_id: "agents.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents/{agentId}/audit_events",
        tag: "ai",
        operation_id: "agents.auditEvents.list",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.create",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
        tag: "ai",
        operation_id: "agents.providerBindings.activate",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.create",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_binding_and_deployment_operations_are_registered() {
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "GET",
            "/app/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.list",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "POST",
            "/app/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.create",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "POST",
            "/app/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
            "agents.providerBindings.activate",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "GET",
            "/app/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.list",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "POST",
            "/app/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.create",
        );

        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "GET",
            "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.list",
        );
        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "POST",
            "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.create",
        );
        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "POST",
            "/backend/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
            "agents.providerBindings.activate",
        );
        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "GET",
            "/backend/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.list",
        );
        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "POST",
            "/backend/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.create",
        );
    }

    #[test]
    fn openapi_specs_expose_provider_binding_and_deployment_contracts() {
        let app_openapi = include_str!("../specs/openapi/agent-business-app-openapi-3.1.2.yaml");
        let backend_openapi =
            include_str!("../specs/openapi/agent-business-backend-openapi-3.1.2.yaml");

        for (label, openapi, prefix) in [
            ("app", app_openapi, "/app/v3/api"),
            ("backend", backend_openapi, "/backend/v3/api"),
        ] {
            for required in [
                format!("{prefix}/ai/agents/{{agentId}}/provider_bindings:"),
                format!("{prefix}/ai/agents/{{agentId}}/provider_bindings/{{bindingId}}/activate:"),
                format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                "- $ref: '#/components/parameters/Page'".to_string(),
                "- $ref: '#/components/parameters/PageSize'".to_string(),
                "operationId: agents.providerBindings.list".to_string(),
                "operationId: agents.providerBindings.create".to_string(),
                "operationId: agents.providerBindings.activate".to_string(),
                "operationId: agents.deployments.list".to_string(),
                "operationId: agents.deployments.create".to_string(),
                "AgentImplementationKind:".to_string(),
                "DeploymentStatus:".to_string(),
                "enum: [created, active, failed, archived]".to_string(),
                "AgentProviderBindingRecord:".to_string(),
                "AgentProviderBindingResponse:".to_string(),
                "AgentProviderBindingListResponse:".to_string(),
                "CreateAgentProviderBindingRequest:".to_string(),
                "ActivateAgentProviderBindingRequest:".to_string(),
                "AgentDeploymentRecord:".to_string(),
                "AgentDeploymentResponse:".to_string(),
                "AgentDeploymentListResponse:".to_string(),
                "CreateAgentDeploymentRequest:".to_string(),
                "implementationProviderId:".to_string(),
                "implementationKind:".to_string(),
                "required: [items, pageInfo]".to_string(),
                "pattern: '^binding\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^profile\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^deployment\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^[a-z0-9_-]+(\\.[a-z0-9_-]+)+$'".to_string(),
                "uniqueItems: true".to_string(),
            ] {
                assert!(
                    openapi.contains(required.as_str()),
                    "{label} OpenAPI must contain {required}"
                );
            }

            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/provider_bindings:"),
                "get:",
                "post:",
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/provider_bindings:"),
                "post:",
                &format!(
                    "{prefix}/ai/agents/{{agentId}}/provider_bindings/{{bindingId}}/activate:"
                ),
                &["'400':", "'403':", "'404':", "'409':"],
            );
            assert_operation_block_excludes(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/provider_bindings:"),
                "post:",
                &format!(
                    "{prefix}/ai/agents/{{agentId}}/provider_bindings/{{bindingId}}/activate:"
                ),
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!(
                    "{prefix}/ai/agents/{{agentId}}/provider_bindings/{{bindingId}}/activate:"
                ),
                "post:",
                &format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                &["'400':", "'403':", "'404':"],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                "get:",
                "post:",
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                "post:",
                "components:",
                &["'400':", "'403':", "'404':", "'409':"],
            );
            assert_operation_block_excludes(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                "post:",
                "components:",
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentRecord:",
                "implementationProviderId:",
                "implementationKind:",
                &["pattern: '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateAgentRequest:",
                "implementationProviderId:",
                "implementationKind:",
                &["pattern: '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
        }

        for required in ["provider_binding_changed", "deployment_created"] {
            assert!(
                backend_openapi.contains(required),
                "backend OpenAPI audit action enum must contain {required}"
            );
        }
    }

    fn assert_operation_block_contains(
        label: &str,
        openapi: &str,
        path: &str,
        operation: &str,
        until: &str,
        required: &[&str],
    ) {
        let block = operation_block(openapi, path, operation, until);
        for item in required {
            assert!(
                block.contains(item),
                "{label} OpenAPI block {path} {operation} must contain {item}"
            );
        }
    }

    fn assert_operation_block_excludes(
        label: &str,
        openapi: &str,
        path: &str,
        operation: &str,
        until: &str,
        forbidden: &[&str],
    ) {
        let block = operation_block(openapi, path, operation, until);
        for item in forbidden {
            assert!(
                !block.contains(item),
                "{label} OpenAPI block {path} {operation} must not contain {item}"
            );
        }
    }

    fn assert_schema_property_block_contains(
        label: &str,
        openapi: &str,
        schema: &str,
        property: &str,
        until: &str,
        required: &[&str],
    ) {
        let schema_start = openapi
            .find(schema)
            .unwrap_or_else(|| panic!("{label} OpenAPI must contain schema {schema}"));
        let after_schema = &openapi[schema_start..];
        let property_start = after_schema
            .find(property)
            .unwrap_or_else(|| panic!("{label} OpenAPI schema {schema} must contain {property}"));
        let after_property = &after_schema[property_start..];
        let end = after_property.find(until).unwrap_or_else(|| {
            panic!("{label} OpenAPI schema {schema} property {property} must end at {until}")
        });
        let block = &after_property[..end];

        for item in required {
            assert!(
                block.contains(item),
                "{label} OpenAPI schema {schema} property {property} must contain {item}"
            );
        }
    }

    fn operation_block<'a>(openapi: &'a str, path: &str, operation: &str, until: &str) -> &'a str {
        let path_start = openapi
            .find(path)
            .unwrap_or_else(|| panic!("OpenAPI must contain path {path}"));
        let after_path = &openapi[path_start..];
        let operation_start = after_path
            .find(operation)
            .unwrap_or_else(|| panic!("OpenAPI path {path} must contain operation {operation}"));
        let after_operation = &after_path[operation_start..];
        let end = after_operation
            .find(until)
            .unwrap_or_else(|| panic!("OpenAPI operation {path} {operation} must end at {until}"));
        &after_operation[..end]
    }

    fn assert_operation(operations: &[ApiOperation], method: &str, path: &str, operation_id: &str) {
        assert!(
            operations.iter().any(|operation| {
                operation.method == method
                    && operation.path == path
                    && operation.operation_id == operation_id
            }),
            "{method} {path} must be registered as {operation_id}"
        );
    }
}
