use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use codex_app_server_client::{
    EnvironmentManager, InProcessAppServerClient, InProcessAppServerRequestHandle,
    InProcessClientStartArgs, InProcessServerEvent, DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
};
use codex_app_server_protocol::{
    ClientRequest, ConfigWarningNotification, JSONRPCErrorError, RequestId, ServerNotification,
    SessionSource, ThreadItemsListParams, ThreadItemsListResponse, ThreadListParams,
    ThreadListResponse, ThreadReadParams, ThreadReadResponse, ThreadTurnsListParams,
    ThreadTurnsListResponse,
};
use codex_core::config::Config;
use sdkwork_agent_kernel::{KernelError, KernelResult};
use sdkwork_agent_provider_core::{
    InMemoryProviderSessionActivityProvider, ProviderSessionActivityAdapter,
};
use tokio::sync::{oneshot, OnceCell};

use crate::{CodexAdapter, CodexThreadActivityObservation};

const CODEX_PROVIDER_ID: &str = "codex";
const APP_SERVER_CLIENT_NAME: &str = "sdkwork-kernel-codex";

#[async_trait]
pub trait CodexThreadClient: Send + Sync {
    async fn list_threads(&self, params: ThreadListParams) -> KernelResult<ThreadListResponse>;

    async fn read_thread(&self, params: ThreadReadParams) -> KernelResult<ThreadReadResponse>;

    async fn list_turns(
        &self,
        params: ThreadTurnsListParams,
    ) -> KernelResult<ThreadTurnsListResponse>;

    async fn list_items(
        &self,
        params: ThreadItemsListParams,
    ) -> KernelResult<ThreadItemsListResponse>;
}

pub struct CodexInProcessThreadClient {
    runtime: OnceCell<Arc<CodexAppServerRuntime>>,
    next_request_id: AtomicI64,
    activity: Arc<InMemoryProviderSessionActivityProvider>,
}

impl CodexInProcessThreadClient {
    pub fn new(activity: Arc<InMemoryProviderSessionActivityProvider>) -> Self {
        Self {
            runtime: OnceCell::new(),
            next_request_id: AtomicI64::new(1),
            activity,
        }
    }

    async fn runtime(&self) -> KernelResult<&Arc<CodexAppServerRuntime>> {
        self.runtime
            .get_or_try_init(|| start_runtime(Arc::clone(&self.activity)))
            .await
    }

    fn request_id(&self) -> RequestId {
        RequestId::Integer(self.next_request_id.fetch_add(1, Ordering::Relaxed))
    }
}

#[async_trait]
impl CodexThreadClient for CodexInProcessThreadClient {
    async fn list_threads(&self, params: ThreadListParams) -> KernelResult<ThreadListResponse> {
        self.runtime()
            .await?
            .requests
            .request_typed(ClientRequest::ThreadList {
                request_id: self.request_id(),
                params,
            })
            .await
            .map_err(app_server_request_error)
    }

    async fn read_thread(&self, params: ThreadReadParams) -> KernelResult<ThreadReadResponse> {
        self.runtime()
            .await?
            .requests
            .request_typed(ClientRequest::ThreadRead {
                request_id: self.request_id(),
                params,
            })
            .await
            .map_err(app_server_request_error)
    }

    async fn list_turns(
        &self,
        params: ThreadTurnsListParams,
    ) -> KernelResult<ThreadTurnsListResponse> {
        self.runtime()
            .await?
            .requests
            .request_typed(ClientRequest::ThreadTurnsList {
                request_id: self.request_id(),
                params,
            })
            .await
            .map_err(app_server_request_error)
    }

    async fn list_items(
        &self,
        params: ThreadItemsListParams,
    ) -> KernelResult<ThreadItemsListResponse> {
        self.runtime()
            .await?
            .requests
            .request_typed(ClientRequest::ThreadItemsList {
                request_id: self.request_id(),
                params,
            })
            .await
            .map_err(app_server_request_error)
    }
}

struct CodexAppServerRuntime {
    requests: InProcessAppServerRequestHandle,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl Drop for CodexAppServerRuntime {
    fn drop(&mut self) {
        if let Ok(mut shutdown_tx) = self.shutdown_tx.lock() {
            if let Some(shutdown_tx) = shutdown_tx.take() {
                let _ = shutdown_tx.send(());
            }
        }
    }
}

async fn start_runtime(
    activity: Arc<InMemoryProviderSessionActivityProvider>,
) -> KernelResult<Arc<CodexAppServerRuntime>> {
    let config = Config::load_with_cli_overrides(Vec::new())
        .await
        .map_err(app_server_start_error)?;
    let state_db = codex_core::init_state_db(&config).await;
    let environment_manager = EnvironmentManager::from_codex_home(
        config.codex_home.clone(),
        None,
        config.http_client_factory(),
    )
    .await
    .map_err(app_server_start_error)?;
    let config_warnings = config
        .startup_warnings
        .iter()
        .map(|warning| ConfigWarningNotification {
            summary: warning.clone(),
            details: None,
            path: None,
            range: None,
        })
        .collect();
    let mut client = InProcessAppServerClient::start(InProcessClientStartArgs {
        arg0_paths: Default::default(),
        config: Arc::new(config),
        cli_overrides: Vec::new(),
        loader_overrides: Default::default(),
        strict_config: false,
        cloud_config_bundle: Default::default(),
        feedback: Default::default(),
        log_db: None,
        state_db,
        environment_manager: Arc::new(environment_manager),
        config_warnings,
        session_source: SessionSource::AppServer.into(),
        enable_codex_api_key_env: false,
        client_name: APP_SERVER_CLIENT_NAME.to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        mcp_server_openai_form_elicitation: false,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
    .await
    .map_err(app_server_start_error)?;
    let requests = client.request_handle();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                event = client.next_event() => {
                    let Some(event) = event else {
                        break;
                    };
                    handle_server_event(&client, event, activity.as_ref()).await;
                }
            }
        }
        let _ = client.shutdown().await;
    });

    Ok(Arc::new(CodexAppServerRuntime {
        requests,
        shutdown_tx: Mutex::new(Some(shutdown_tx)),
    }))
}

async fn handle_server_event(
    client: &InProcessAppServerClient,
    event: InProcessServerEvent,
    activity: &InMemoryProviderSessionActivityProvider,
) {
    match event {
        InProcessServerEvent::ServerNotification(notification) => {
            if let ServerNotification::ThreadStatusChanged(notification) = *notification {
                let observation = CodexThreadActivityObservation::from_protocol(
                    notification.thread_id,
                    notification.status,
                );
                if let Ok(snapshot) = CodexAdapter::new().to_session_activity(&observation) {
                    let _ = activity.record(snapshot);
                }
            }
        }
        InProcessServerEvent::ServerRequest(request) => {
            let _ = client
                .reject_server_request(
                    request.id().clone(),
                    JSONRPCErrorError {
                        code: -32000,
                        message: "SDKWork Codex history client does not handle interactive server requests"
                            .to_string(),
                        data: None,
                    },
                )
                .await;
        }
        InProcessServerEvent::Lagged { .. } => {}
    }
}

fn app_server_start_error(error: impl std::fmt::Display) -> KernelError {
    KernelError::provider_error(
        "codex_app_server_start_failed",
        format!("failed to start Codex app-server client: {error}"),
    )
    .with_provider(CODEX_PROVIDER_ID)
    .with_safe_message("Codex history is currently unavailable")
}

fn app_server_request_error(error: impl std::fmt::Display) -> KernelError {
    KernelError::provider_error(
        "codex_app_server_request_failed",
        format!("Codex app-server request failed: {error}"),
    )
    .with_provider(CODEX_PROVIDER_ID)
    .with_safe_message("Codex history request failed")
}
