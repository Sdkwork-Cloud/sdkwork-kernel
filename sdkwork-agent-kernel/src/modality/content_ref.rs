//! Canonical content references for multimodal parts — API-neutral URI contract.
//!
//! Protocol adapters (OpenAI, A2A, internal-api, …) map wire payloads into
//! `ContentReference`; model providers map `ContentReference` into vendor wire.

use super::kind::{infer_modality_from_mime_type, AgentInputModality};
use crate::{KernelError, KernelResult};

pub const SCHEME_HOST: &str = "host";
pub const SCHEME_ARTIFACT: &str = "artifact";
pub const SCHEME_DRIVE: &str = "drive";
pub const SCHEME_HTTPS: &str = "https";
pub const SCHEME_HTTP: &str = "http";
pub const SCHEME_INLINE: &str = "inline";

/// Storage / retrieval scheme for referenced media — not a vendor API shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentReferenceScheme {
    Host,
    Artifact,
    Drive,
    Https,
    Http,
    Inline,
}

impl ContentReferenceScheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Host => SCHEME_HOST,
            Self::Artifact => SCHEME_ARTIFACT,
            Self::Drive => SCHEME_DRIVE,
            Self::Https => SCHEME_HTTPS,
            Self::Http => SCHEME_HTTP,
            Self::Inline => SCHEME_INLINE,
        }
    }

    pub fn parse(scheme: &str) -> KernelResult<Self> {
        match scheme {
            SCHEME_HOST => Ok(Self::Host),
            SCHEME_ARTIFACT => Ok(Self::Artifact),
            SCHEME_DRIVE => Ok(Self::Drive),
            SCHEME_HTTPS => Ok(Self::Https),
            SCHEME_HTTP => Ok(Self::Http),
            SCHEME_INLINE => Ok(Self::Inline),
            _ => Err(KernelError::validation(format!(
                "unknown content reference scheme: {scheme}"
            ))),
        }
    }

    pub fn requires_policy_for_fetch(self) -> bool {
        matches!(self, Self::Https | Self::Http | Self::Drive)
    }

    pub fn validate_uri(&self, uri: &str) -> KernelResult<()> {
        if self.requires_policy_for_fetch() && !sdkwork_utils_rust::validation::is_url(uri) {
            return Err(KernelError::validation(format!(
                "content reference uri must be a valid URL for scheme {}: {uri}",
                self.as_str()
            )));
        }
        Ok(())
    }
}

/// Kernel-neutral pointer to bytes accessible through host/artifact/drive providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentReference {
    pub scheme: ContentReferenceScheme,
    pub uri: String,
}

impl ContentReference {
    pub fn new(scheme: ContentReferenceScheme, uri: impl Into<String>) -> Self {
        Self {
            scheme,
            uri: uri.into(),
        }
    }

    pub fn host(path: impl Into<String>) -> Self {
        let path = path.into();
        let uri = if path.contains("://") {
            path
        } else {
            format!("{SCHEME_HOST}://{path}")
        };
        match Self::parse(&uri) {
            Ok(reference) => reference,
            Err(_) => Self {
                scheme: ContentReferenceScheme::Host,
                uri,
            },
        }
    }

    pub fn artifact(artifact_id: impl Into<String>) -> Self {
        let id = artifact_id.into();
        Self {
            scheme: ContentReferenceScheme::Artifact,
            uri: format!("{SCHEME_ARTIFACT}://{id}"),
        }
    }

    pub fn parse(uri: &str) -> KernelResult<Self> {
        let trimmed = uri.trim();
        let Some((scheme, rest)) = trimmed.split_once("://") else {
            return Err(KernelError::validation(format!(
                "content reference must use scheme:// form: {uri}"
            )));
        };
        if rest.is_empty() {
            return Err(KernelError::validation(format!(
                "content reference path must not be empty: {uri}"
            )));
        }
        let reference = Self {
            scheme: ContentReferenceScheme::parse(scheme)?,
            uri: trimmed.to_string(),
        };
        reference.scheme.validate_uri(trimmed)?;
        Ok(reference)
    }

    pub fn inferred_modality(&self, mime_type: Option<&str>) -> Option<AgentInputModality> {
        mime_type.and_then(infer_modality_from_mime_type)
    }
}
