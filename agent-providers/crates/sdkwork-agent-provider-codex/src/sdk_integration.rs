use crate::{
    map_thread_page, map_thread_record, map_turn_page, normalize_page_limit, CodexAdapter,
    CodexInProcessThreadClient, CodexLifecycleProvider, CodexMessageAdapter, CodexMessagePage,
    CodexModelProvider, CodexSessionPage, CodexSessionRecord, CodexSortDirection,
    CodexThreadActivityObservation, CodexThreadClient, ThreadItemsListParams,
    ThreadItemsListResponse, ThreadListParams, ThreadListResponse, ThreadReadParams,
    ThreadReadResponse, ThreadTurnsListParams, ThreadTurnsListResponse, TurnItemsView,
};
use sdkwork_agent_kernel::{
    KernelResult, ProviderSessionActivityProvider, SessionActivitySnapshot,
};
use sdkwork_agent_provider_core::{
    InMemoryProviderSessionActivityProvider, ProviderSessionActivityAdapter,
};
use sdkwork_agent_provider_spi::{
    bootstrap_binding, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkNegotiationError, SdkRuntimeBackedModelProvider,
    SdkRuntimeBackedSessionControlProvider, SdkRuntimeInteractionResolution, SdkRuntimeRequest,
    SdkRuntimeResponse, SdkRuntimeRouter, CODEX_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
};
use sdkwork_agent_provider_transport_core::{
    ProviderTransportBootstrap, ProviderTransportRegistry, RustNativeTransportHost,
};
use std::sync::Arc;

const CODEX_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/codex/provider-binding.manifest.json");

pub fn codex_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(CODEX_BINDING_MANIFEST_JSON)
        .expect("codex provider binding manifest must parse")
}

pub struct CodexSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: CodexLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub session_control: SdkRuntimeBackedSessionControlProvider,
    pub session_adapter: CodexAdapter,
    pub message_adapter: CodexMessageAdapter,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
    thread_client: Arc<dyn CodexThreadClient>,
}

impl CodexSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let activity = Arc::new(InMemoryProviderSessionActivityProvider::new());
        let thread_client = Arc::new(CodexInProcessThreadClient::new(activity.clone()));
        Self::bootstrap_with_components(activity, thread_client)
    }

    pub fn bootstrap_with_thread_client(
        thread_client: Arc<dyn CodexThreadClient>,
    ) -> Result<Self, SdkNegotiationError> {
        Self::bootstrap_with_components(
            Arc::new(InMemoryProviderSessionActivityProvider::new()),
            thread_client,
        )
    }

    fn bootstrap_with_components(
        activity: Arc<InMemoryProviderSessionActivityProvider>,
        thread_client: Arc<dyn CodexThreadClient>,
    ) -> Result<Self, SdkNegotiationError> {
        let manifest = codex_binding_manifest();
        let mut drivers = DriverRegistry::new();
        let mut bindings = BindingRegistry::new();
        let negotiation = bootstrap_binding(manifest, &mut drivers, &mut bindings)?;

        let inner_model = Arc::new(CodexModelProvider::new());
        let app_server_runtime = Arc::new(CodexInProcessThreadClient::new(activity.clone()));
        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(RustNativeTransportHost::new(
            "codex-app-server-client",
        )));
        bootstrap.with_rust_runtime(app_server_runtime);
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;

        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            inner_model,
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.codex",
        );
        let session_control = SdkRuntimeBackedSessionControlProvider::new(
            runtime.clone(),
            crate::ids::SESSION_CONTROL_PROVIDER_ID,
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: CodexLifecycleProvider::new(),
            model,
            session_control,
            session_adapter: CodexAdapter::new(),
            message_adapter: CodexMessageAdapter::new(),
            activity,
            thread_client,
        })
    }

    pub fn binding_id(&self) -> &str {
        CODEX_BINDING_ID
    }

    pub async fn list_codex_threads(
        &self,
        mut params: ThreadListParams,
    ) -> KernelResult<ThreadListResponse> {
        normalize_page_limit(&mut params.limit)?;
        self.thread_client.list_threads(params).await
    }

    pub async fn list_provider_sessions(
        &self,
        params: ThreadListParams,
    ) -> KernelResult<CodexSessionPage> {
        map_thread_page(self.list_codex_threads(params).await?)
    }

    pub async fn read_codex_thread(
        &self,
        params: ThreadReadParams,
    ) -> KernelResult<ThreadReadResponse> {
        self.thread_client.read_thread(params).await
    }

    pub async fn read_provider_session(
        &self,
        params: ThreadReadParams,
    ) -> KernelResult<CodexSessionRecord> {
        map_thread_record(self.read_codex_thread(params).await?.thread)
    }

    pub async fn list_provider_session_turns(
        &self,
        mut params: ThreadTurnsListParams,
    ) -> KernelResult<ThreadTurnsListResponse> {
        normalize_page_limit(&mut params.limit)?;
        self.thread_client.list_turns(params).await
    }

    pub async fn list_codex_thread_items(
        &self,
        mut params: ThreadItemsListParams,
    ) -> KernelResult<ThreadItemsListResponse> {
        normalize_page_limit(&mut params.limit)?;
        self.thread_client.list_items(params).await
    }

    pub async fn get_provider_session_history(
        &self,
        mut params: ThreadTurnsListParams,
    ) -> KernelResult<CodexMessagePage> {
        let thread_id = params.thread_id.clone();
        normalize_page_limit(&mut params.limit)?;
        params.sort_direction = Some(params.sort_direction.unwrap_or(CodexSortDirection::Asc));
        params.items_view = Some(TurnItemsView::Full);
        map_turn_page(&thread_id, self.thread_client.list_turns(params).await?)
    }

    pub fn codex_thread_client(&self) -> Arc<dyn CodexThreadClient> {
        self.thread_client.clone()
    }

    pub fn invoke_runtime(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_provider_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }

    pub fn resolve_interaction(
        &self,
        resolution: &SdkRuntimeInteractionResolution,
    ) -> Result<serde_json::Value, sdkwork_agent_provider_spi::SdkRuntimeError> {
        self.runtime
            .resolve_interaction(SDK_CAPABILITY_MODEL_CHAT, resolution)
    }

    /// Records one status returned by the live Codex app-server thread API.
    pub fn record_provider_session_activity(
        &self,
        observation: &CodexThreadActivityObservation,
    ) -> KernelResult<SessionActivitySnapshot> {
        self.activity
            .record(self.session_adapter.to_session_activity(observation)?)
    }

    pub fn provider_session_activity_provider(&self) -> Arc<dyn ProviderSessionActivityProvider> {
        self.activity.clone()
    }
}

impl ProviderSessionActivityProvider for CodexSdkIntegration {
    fn get_provider_session_activity(
        &self,
        provider_session_id: &str,
    ) -> KernelResult<SessionActivitySnapshot> {
        self.activity
            .get_provider_session_activity(provider_session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{
        KernelError, ModelProvider, ModelRequest, SessionActivityFreshness,
        SessionActivityInteractionHint, SessionActivityState,
    };
    use sdkwork_agent_provider_spi::SdkBackendKind;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeCodexThreadClient {
        thread_list_params: Mutex<Vec<ThreadListParams>>,
        turn_list_params: Mutex<Vec<ThreadTurnsListParams>>,
        item_list_params: Mutex<Vec<ThreadItemsListParams>>,
    }

    #[async_trait::async_trait]
    impl CodexThreadClient for FakeCodexThreadClient {
        async fn list_threads(&self, params: ThreadListParams) -> KernelResult<ThreadListResponse> {
            self.thread_list_params
                .lock()
                .expect("thread params lock")
                .push(params);
            Ok(ThreadListResponse {
                data: Vec::new(),
                next_cursor: Some("thread-next".to_string()),
                backwards_cursor: Some("thread-backwards".to_string()),
            })
        }

        async fn read_thread(&self, _params: ThreadReadParams) -> KernelResult<ThreadReadResponse> {
            Err(KernelError::provider_error(
                "fake_read_not_configured",
                "fake read response is not configured",
            ))
        }

        async fn list_turns(
            &self,
            params: ThreadTurnsListParams,
        ) -> KernelResult<ThreadTurnsListResponse> {
            self.turn_list_params
                .lock()
                .expect("turn params lock")
                .push(params);
            Ok(ThreadTurnsListResponse {
                data: Vec::new(),
                next_cursor: Some("turn-next".to_string()),
                backwards_cursor: Some("turn-backwards".to_string()),
            })
        }

        async fn list_items(
            &self,
            params: ThreadItemsListParams,
        ) -> KernelResult<ThreadItemsListResponse> {
            self.item_list_params
                .lock()
                .expect("item params lock")
                .push(params);
            Ok(ThreadItemsListResponse {
                data: vec![crate::ThreadItemEntry {
                    turn_id: "turn-1".to_string(),
                    item: crate::ThreadItem::ContextCompaction {
                        id: "item-1".to_string(),
                    },
                }],
                next_cursor: Some("item-next".to_string()),
                backwards_cursor: Some("item-backwards".to_string()),
            })
        }
    }

    #[test]
    fn bootstrap_negotiates_required_codex_capabilities() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), CODEX_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::RustNative)
        );
        assert_eq!(
            integration.sdk.selected_driver_id("sdk.session.lifecycle"),
            Some("driver.codex.session.lifecycle.rust")
        );
    }

    #[test]
    fn exposes_kernel_providers_after_bootstrap() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert!(!integration.model.list_models().is_empty());
    }

    #[test]
    fn runtime_model_chat_uses_rust_app_server_backend() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .invoke_runtime(&SdkRuntimeRequest::model_chat(
                "sdk.model.chat",
                "req-runtime-1",
                vec!["hello codex".to_string()],
            ))
            .expect("runtime invoke should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::RustNative);
    }

    #[test]
    fn model_provider_routes_invoke_through_runtime() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-kernel-1", vec!["hello".to_string()]))
            .expect("model invoke should succeed");
        assert!(response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("sdk_runtime_mode=")));
        assert!(!response
            .messages
            .iter()
            .any(|message| message.contains("Mock response")));
    }

    #[test]
    fn runtime_activity_capability_records_queries_and_expires_codex_status() {
        let integration = CodexSdkIntegration::bootstrap().expect("bootstrap should succeed");
        integration
            .record_provider_session_activity(&CodexThreadActivityObservation {
                provider_session_id: "codex.provider.1".to_string(),
                status: crate::ThreadStatus::Active {
                    active_flags: vec![crate::ThreadActiveFlag::WaitingOnApproval],
                },
                observed_at: sdkwork_agent_provider_core::now_iso(),
            })
            .expect("record activity");
        let waiting = integration
            .get_provider_session_activity("codex.provider.1")
            .expect("query activity");
        integration
            .record_provider_session_activity(&CodexThreadActivityObservation {
                provider_session_id: "codex.provider.stale".to_string(),
                status: crate::ThreadStatus::Active {
                    active_flags: Vec::new(),
                },
                observed_at: "2000-01-01T00:00:00Z".to_string(),
            })
            .expect("record stale activity");
        let stale = integration
            .get_provider_session_activity("codex.provider.stale")
            .expect("query stale activity");
        let unknown = integration
            .get_provider_session_activity("codex.provider.unknown")
            .expect("query unknown activity");

        assert_eq!(waiting.state, Some(SessionActivityState::Waiting));
        assert_eq!(
            waiting.interaction_hint,
            Some(SessionActivityInteractionHint::ApprovalRequired)
        );
        assert_eq!(stale.freshness, SessionActivityFreshness::Stale);
        assert_eq!(unknown.freshness, SessionActivityFreshness::Unsupported);
    }

    #[tokio::test]
    async fn delegates_typed_pagination_without_rewriting_cursors() {
        let client = Arc::new(FakeCodexThreadClient::default());
        let integration = CodexSdkIntegration::bootstrap_with_thread_client(client.clone())
            .expect("fake client bootstrap");
        let thread_params: ThreadListParams = serde_json::from_value(serde_json::json!({
            "cursor": "thread-input"
        }))
        .expect("thread list params");
        let thread_page = integration
            .list_provider_sessions(thread_params)
            .await
            .expect("thread list");
        let turn_page = integration
            .list_provider_session_turns(ThreadTurnsListParams {
                thread_id: "thread-1".to_string(),
                cursor: Some("turn-input".to_string()),
                limit: Some(sdkwork_utils_rust::MAX_LIST_PAGE_SIZE as u32),
                sort_direction: None,
                items_view: None,
            })
            .await
            .expect("turn list");
        let item_page = integration
            .list_codex_thread_items(ThreadItemsListParams {
                thread_id: "thread-1".to_string(),
                turn_id: Some("turn-1".to_string()),
                cursor: Some("item-input".to_string()),
                limit: None,
                sort_direction: None,
            })
            .await
            .expect("item list");
        let history_page = integration
            .get_provider_session_history(ThreadTurnsListParams {
                thread_id: "thread-1".to_string(),
                cursor: Some("history-input".to_string()),
                limit: None,
                sort_direction: None,
                items_view: None,
            })
            .await
            .expect("history list");

        assert_eq!(thread_page.next_cursor.as_deref(), Some("thread-next"));
        assert_eq!(
            thread_page.backwards_cursor.as_deref(),
            Some("thread-backwards")
        );
        assert_eq!(turn_page.next_cursor.as_deref(), Some("turn-next"));
        assert_eq!(
            turn_page.backwards_cursor.as_deref(),
            Some("turn-backwards")
        );
        assert_eq!(item_page.next_cursor.as_deref(), Some("item-next"));
        assert_eq!(
            item_page.backwards_cursor.as_deref(),
            Some("item-backwards")
        );
        assert_eq!(item_page.data[0].item.id(), "item-1");
        assert_eq!(history_page.next_cursor.as_deref(), Some("turn-next"));

        let thread_params = client
            .thread_list_params
            .lock()
            .expect("thread params lock");
        assert_eq!(thread_params[0].cursor.as_deref(), Some("thread-input"));
        assert_eq!(
            thread_params[0].limit,
            Some(sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE as u32)
        );
        let turn_params = client.turn_list_params.lock().expect("turn params lock");
        assert_eq!(turn_params[0].cursor.as_deref(), Some("turn-input"));
        assert_eq!(
            turn_params[0].limit,
            Some(sdkwork_utils_rust::MAX_LIST_PAGE_SIZE as u32)
        );
        assert_eq!(turn_params[1].cursor.as_deref(), Some("history-input"));
        assert_eq!(
            turn_params[1].limit,
            Some(sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE as u32)
        );
        assert_eq!(
            turn_params[1].sort_direction,
            Some(crate::CodexSortDirection::Asc)
        );
        assert_eq!(turn_params[1].items_view, Some(crate::TurnItemsView::Full));
        let item_params = client.item_list_params.lock().expect("item params lock");
        assert_eq!(item_params[0].cursor.as_deref(), Some("item-input"));
        assert_eq!(
            item_params[0].limit,
            Some(sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE as u32)
        );
    }
}
