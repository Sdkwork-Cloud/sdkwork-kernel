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
];
