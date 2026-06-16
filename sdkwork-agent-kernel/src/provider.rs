use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest};

/// 统一的 Provider 基础接口
///
/// 所有 Agent Kernel Provider 都应实现此 trait。
/// 它定义了 provider 的基本契约：清单声明和健康检查。
pub trait AgentProvider {
    /// 返回 provider 的清单信息
    fn provider_manifest(&self) -> ProviderManifest;

    /// 返回 provider 的健康状态
    ///
    /// 默认实现返回 `ProviderHealth::available()`。
    /// 需要自定义健康检查的 provider 应覆盖此方法。
    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    /// 返回 provider 支持的能力列表
    ///
    /// 默认从 provider_manifest() 中提取 capabilities。
    fn capabilities(&self) -> Vec<String> {
        self.provider_manifest().capabilities
    }

    /// 检查 provider 是否支持指定能力
    fn supports_capability(&self, capability_id: &str) -> bool {
        self.capabilities().iter().any(|c| c == capability_id)
    }
}

/// 可取消的操作 trait
///
/// 支持取消正在执行的操作的 provider 应实现此 trait。
pub trait Cancellable {
    /// 操作结果类型
    type Result;

    /// 取消指定操作
    ///
    /// # 参数
    /// - `operation_id`: 要取消的操作 ID
    ///
    /// # 返回
    /// 取消操作的结果，如果操作不存在或已完成则返回错误。
    fn cancel(&self, operation_id: &str) -> KernelResult<Self::Result>;
}

/// 支持流式输出的操作 trait
///
/// 支持流式输出的 provider 应实现此 trait。
pub trait Streaming {
    /// 流式输出的 chunk 类型
    type Chunk;

    /// 流式输出的请求类型
    type Request;

    /// 启动流式输出
    ///
    /// # 参数
    /// - `request`: 流式请求
    ///
    /// # 返回
    /// 流式输出的 chunk 列表。对于真正的流式实现，
    /// 应返回一个迭代器或通道。
    fn stream(&self, request: Self::Request) -> KernelResult<Vec<Self::Chunk>>;
}

/// 需要策略评估的操作 trait
///
/// 需要策略评估的操作应实现此 trait。
pub trait PolicyGated {
    /// 返回是否需要策略评估
    fn requires_policy(&self) -> bool;

    /// 返回策略类别列表
    fn policy_categories(&self) -> Vec<String>;
}

/// 支持目录列举的 provider trait
///
/// 支持列举资源的 provider 应实现此 trait。
pub trait Listable<T> {
    /// 列举所有资源
    fn list(&self) -> KernelResult<Vec<T>>;

    /// 根据 ID 获取单个资源
    fn get(&self, id: &str) -> KernelResult<T>;

    /// 检查资源是否存在
    fn exists(&self, id: &str) -> bool {
        self.get(id).is_ok()
    }
}

/// 支持生命周期管理的 provider trait
///
/// 支持创建、更新、删除操作的 provider 应实现此 trait。
pub trait Lifecycle<C, U> {
    /// 资源类型
    type Resource;
    /// 创建结果类型
    type CreateResult;
    /// 更新结果类型
    type UpdateResult;
    /// 删除结果类型
    type DeleteResult;

    /// 创建资源
    fn create(&self, command: C) -> KernelResult<Self::CreateResult>;

    /// 更新资源
    fn update(&self, command: U) -> KernelResult<Self::UpdateResult>;

    /// 删除资源
    fn delete(&self, id: &str) -> KernelResult<Self::DeleteResult>;
}

/// 支持批量操作的 provider trait
///
/// 支持批量操作的 provider 应实现此 trait。
pub trait BatchOperations<T> {
    /// 批量操作结果类型
    type Result;

    /// 批量创建
    fn create_batch(&self, items: Vec<T>) -> KernelResult<Self::Result>;

    /// 批量删除
    fn delete_batch(&self, ids: Vec<String>) -> KernelResult<Self::Result>;
}

/// Provider 能力声明宏
///
/// 用于简化 provider 能力声明。
///
/// # 示例
///
/// ```rust
/// use sdkwork_agent_kernel::provider_capabilities;
///
/// let capabilities = provider_capabilities! {
///     "model.chat" => "模型对话",
///     "model.stream" => "流式输出",
///     "model.cancel" => "取消调用",
/// };
/// assert_eq!(capabilities.len(), 3);
/// ```
#[macro_export]
macro_rules! provider_capabilities {
    ($($capability:expr => $description:expr),* $(,)?) => {
        vec![
            $($capability.to_string()),*
        ]
    };
}

/// Provider 健康检查宏
///
/// 用于简化 provider 健康检查实现。
///
/// # 示例
///
/// ```rust
/// use sdkwork_agent_kernel::provider_health;
///
/// let health = provider_health!("available");
/// assert_eq!(health.status, "available");
///
/// let degraded = provider_health!("degraded");
/// assert_eq!(degraded.status, "degraded");
/// ```
#[macro_export]
macro_rules! provider_health {
    ("available") => {
        $crate::ProviderHealth::available()
    };
    ($status:expr) => {
        $crate::ProviderHealth {
            status: $status.to_string(),
        }
    };
}

/// Provider 错误类型
///
/// 定义了 provider 常见的错误类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// 能力不支持
    CapabilityNotSupported {
        capability_id: String,
        provider_id: String,
    },
    /// 资源不存在
    ResourceNotFound {
        resource_id: String,
        resource_type: String,
    },
    /// 操作被取消
    OperationCancelled { operation_id: String },
    /// 操作超时
    OperationTimeout {
        operation_id: String,
        timeout_ms: u64,
    },
    /// 策略被拒绝
    PolicyDenied { reason: String },
    /// 配置错误
    ConfigurationError { message: String },
    /// 内部错误
    InternalError { message: String },
}

impl ProviderError {
    /// 创建能力不支持错误
    pub fn capability_not_supported(
        capability_id: impl Into<String>,
        provider_id: impl Into<String>,
    ) -> Self {
        Self::CapabilityNotSupported {
            capability_id: capability_id.into(),
            provider_id: provider_id.into(),
        }
    }

    /// 创建资源不存在错误
    pub fn resource_not_found(
        resource_id: impl Into<String>,
        resource_type: impl Into<String>,
    ) -> Self {
        Self::ResourceNotFound {
            resource_id: resource_id.into(),
            resource_type: resource_type.into(),
        }
    }

    /// 创建操作被取消错误
    pub fn operation_cancelled(operation_id: impl Into<String>) -> Self {
        Self::OperationCancelled {
            operation_id: operation_id.into(),
        }
    }

    /// 创建操作超时错误
    pub fn operation_timeout(operation_id: impl Into<String>, timeout_ms: u64) -> Self {
        Self::OperationTimeout {
            operation_id: operation_id.into(),
            timeout_ms,
        }
    }

    /// 创建策略被拒绝错误
    pub fn policy_denied(reason: impl Into<String>) -> Self {
        Self::PolicyDenied {
            reason: reason.into(),
        }
    }

    /// 创建配置错误
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// 创建内部错误
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapabilityNotSupported {
                capability_id,
                provider_id,
            } => write!(
                f,
                "Capability '{}' not supported by provider '{}'",
                capability_id, provider_id
            ),
            Self::ResourceNotFound {
                resource_id,
                resource_type,
            } => write!(f, "{} '{}' not found", resource_type, resource_id),
            Self::OperationCancelled { operation_id } => {
                write!(f, "Operation '{}' cancelled", operation_id)
            }
            Self::OperationTimeout {
                operation_id,
                timeout_ms,
            } => write!(
                f,
                "Operation '{}' timed out after {}ms",
                operation_id, timeout_ms
            ),
            Self::PolicyDenied { reason } => write!(f, "Policy denied: {}", reason),
            Self::ConfigurationError { message } => write!(f, "Configuration error: {}", message),
            Self::InternalError { message } => write!(f, "Internal error: {}", message),
        }
    }
}

impl std::error::Error for ProviderError {}

/// 从 ProviderError 转换为 KernelError
impl From<ProviderError> for KernelError {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::CapabilityNotSupported { capability_id, .. } => {
                KernelError::CapabilityMissing { capability_id }
            }
            ProviderError::ResourceNotFound { resource_id, .. } => {
                KernelError::validation(format!("Resource not found: {}", resource_id))
            }
            ProviderError::OperationCancelled { operation_id } => {
                KernelError::cancelled(format!("Operation '{}' cancelled", operation_id))
            }
            ProviderError::OperationTimeout {
                operation_id,
                timeout_ms,
            } => KernelError::timeout(format!(
                "Operation '{}' timed out after {}ms",
                operation_id, timeout_ms
            )),
            ProviderError::PolicyDenied { reason } => KernelError::PolicyDenied {
                reason_code: reason,
            },
            ProviderError::ConfigurationError { message } => KernelError::validation(message),
            ProviderError::InternalError { message } => KernelError::Internal { message },
        }
    }
}

/// Provider 注册信息
///
/// 用于在运行时注册 provider 时携带元数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRegistration {
    /// Provider ID
    pub provider_id: String,
    /// Provider 家族
    pub provider_family: String,
    /// Provider 名称
    pub name: String,
    /// Provider 版本
    pub version: String,
    /// 支持的能力列表
    pub capabilities: Vec<String>,
    /// 是否为 typed provider（而非 manifest-only）
    pub typed: bool,
    /// Provider 来源
    pub source: ProviderSource,
}

/// Provider 来源
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSource {
    /// 本地实现
    Local,
    /// 进程适配器
    ProcessAdapter,
    /// 协议适配器
    ProtocolAdapter,
    /// 插件
    Plugin { plugin_id: String },
}

impl ProviderRegistration {
    /// 创建新的 provider 注册信息
    pub fn new(
        provider_id: impl Into<String>,
        provider_family: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            provider_family: provider_family.into(),
            name: name.into(),
            version: version.into(),
            capabilities: Vec::new(),
            typed: true,
            source: ProviderSource::Local,
        }
    }

    /// 添加能力
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    /// 设置为 manifest-only provider
    pub fn manifest_only(mut self) -> Self {
        self.typed = false;
        self
    }

    /// 设置 provider 来源
    pub fn with_source(mut self, source: ProviderSource) -> Self {
        self.source = source;
        self
    }

    /// 转换为 ProviderManifest
    pub fn to_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            &self.provider_id,
            &self.provider_family,
            &self.name,
            &self.version,
            self.capabilities.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_error_to_kernel_error_conversion() {
        let error = ProviderError::capability_not_supported("model.stream", "provider.model.test");
        let kernel_error: KernelError = error.into();
        assert!(matches!(
            kernel_error,
            KernelError::CapabilityMissing { .. }
        ));
    }

    #[test]
    fn provider_registration_to_manifest() {
        let registration = ProviderRegistration::new(
            "provider.model.test",
            "model",
            "Test Model Provider",
            "1.0.0",
        )
        .with_capability("model.chat")
        .with_capability("model.stream");

        let manifest = registration.to_manifest();
        assert_eq!(manifest.provider_id, "provider.model.test");
        assert_eq!(manifest.provider_family, "model");
        assert_eq!(manifest.capabilities.len(), 2);
    }

    #[test]
    fn provider_health_macro() {
        let health = provider_health!("available");
        assert_eq!(health.status, "available");

        let health = provider_health!("degraded");
        assert_eq!(health.status, "degraded");
    }
}
