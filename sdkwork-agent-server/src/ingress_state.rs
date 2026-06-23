use std::sync::Arc;

use crate::config::ServerConfig;
use crate::ingress_jwt::IngressJwtValidator;

/// Shared ingress middleware state (config + optional JWT validator).
#[derive(Clone)]
pub struct IngressMiddlewareState {
    pub config: Arc<ServerConfig>,
    pub jwt_validator: Option<Arc<IngressJwtValidator>>,
}

impl IngressMiddlewareState {
    pub fn from_config(config: Arc<ServerConfig>) -> Result<Self, String> {
        let jwt_validator = if config.ingress_auth_mode.eq_ignore_ascii_case("jwt") {
            Some(Arc::new(IngressJwtValidator::from_config(config.as_ref())?))
        } else {
            None
        };
        Ok(Self {
            config,
            jwt_validator,
        })
    }
}
