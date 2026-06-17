use crate::types::SessionInfo;

/// Query for listing bridge sessions.
#[derive(Debug, Clone, Default)]
pub struct BridgeSessionQuery {
    pub agent_id: Option<String>,
    pub provider_id: Option<String>,
    pub bridge_id: Option<String>,
    pub active_only: bool,
    pub limit: Option<u32>,
}

pub fn sort_bridge_sessions(sessions: &mut [SessionInfo]) {
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
}
