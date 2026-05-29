use crate::Workspace;
use sdkwork_agent_kernel::{KernelResult, ProviderHealth};

pub trait LanguageProvider {
    fn diagnostics(
        &self,
        workspace: &Workspace,
        request: LanguageDiagnosticsRequest,
    ) -> KernelResult<Vec<LanguageDiagnostic>>;

    fn symbols(
        &self,
        workspace: &Workspace,
        request: LanguageSymbolsRequest,
    ) -> KernelResult<Vec<LanguageSymbol>>;

    fn format(
        &self,
        workspace: &Workspace,
        request: LanguageFormatRequest,
    ) -> KernelResult<LanguageFormatResult>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageDiagnosticsRequest {
    pub path: String,
    pub language_id: Option<String>,
    pub include_generated: bool,
}

impl LanguageDiagnosticsRequest {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            language_id: None,
            include_generated: false,
        }
    }

    pub fn with_language_id(mut self, language_id: impl Into<String>) -> Self {
        self.language_id = Some(language_id.into());
        self
    }

    pub fn include_generated(mut self) -> Self {
        self.include_generated = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSymbolsRequest {
    pub path: String,
    pub query: Option<String>,
    pub language_id: Option<String>,
}

impl LanguageSymbolsRequest {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            query: None,
            language_id: None,
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn with_language_id(mut self, language_id: impl Into<String>) -> Self {
        self.language_id = Some(language_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageFormatRequest {
    pub path: String,
    pub content: String,
    pub language_id: Option<String>,
}

impl LanguageFormatRequest {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
            language_id: None,
        }
    }

    pub fn with_language_id(mut self, language_id: impl Into<String>) -> Self {
        self.language_id = Some(language_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageFormatResult {
    pub path: String,
    pub formatted_content: String,
    pub changed: bool,
}

impl LanguageFormatResult {
    pub fn changed(path: impl Into<String>, formatted_content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            formatted_content: formatted_content.into(),
            changed: true,
        }
    }

    pub fn unchanged(path: impl Into<String>, formatted_content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            formatted_content: formatted_content.into(),
            changed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageDiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageDiagnostic {
    pub severity: LanguageDiagnosticSeverity,
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
    pub message: String,
    pub code: Option<String>,
    pub source: Option<String>,
}

impl LanguageDiagnostic {
    pub fn new(
        severity: LanguageDiagnosticSeverity,
        path: impl Into<String>,
        line: u32,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            path: path.into(),
            line,
            column: None,
            message: message.into(),
            code: None,
            source: None,
        }
    }

    pub fn at_column(mut self, column: u32) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageSymbolKind {
    Module,
    Namespace,
    Package,
    Class,
    Interface,
    Struct,
    Enum,
    Function,
    Method,
    Field,
    Variable,
    Constant,
    Trait,
    TypeAlias,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSymbol {
    pub name: String,
    pub kind: LanguageSymbolKind,
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
    pub container_name: Option<String>,
    pub signature: Option<String>,
}

impl LanguageSymbol {
    pub fn new(
        name: impl Into<String>,
        kind: LanguageSymbolKind,
        path: impl Into<String>,
        line: u32,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            path: path.into(),
            line,
            column: None,
            container_name: None,
            signature: None,
        }
    }

    pub fn at_column(mut self, column: u32) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_container_name(mut self, container_name: impl Into<String>) -> Self {
        self.container_name = Some(container_name.into());
        self
    }

    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }
}
