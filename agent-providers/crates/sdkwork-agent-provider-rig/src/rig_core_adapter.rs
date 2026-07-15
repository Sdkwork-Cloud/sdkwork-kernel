use std::{
    future::IntoFuture,
    sync::{mpsc, Arc},
    time::Duration,
};

use rig_core::client::CompletionClient;
use rig_core::completion::Prompt;
use rig_core::providers::openai;
use sdkwork_agent_kernel::{
    HostProvider, KernelError, KernelResult, KnowledgeDocument, KnowledgeSearchRequest,
    ModelRequest, ModelResponse, SecretRef,
};

use crate::{backend::RigBackendExecutor, ids, provider::RigKnowledgeProvider};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigVectorSearchPlan {
    pub query: String,
    pub samples: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RigCoreKnowledgeAdapter;

impl RigCoreKnowledgeAdapter {
    pub fn vector_search_plan(request: &KnowledgeSearchRequest) -> RigVectorSearchPlan {
        RigVectorSearchPlan {
            query: request.query.clone(),
            samples: request.top_k as u64,
        }
    }

    pub fn provider_from_documents(
        documents: impl IntoIterator<Item = KnowledgeDocument>,
    ) -> RigKnowledgeProvider {
        documents
            .into_iter()
            .fold(RigKnowledgeProvider::new(), |provider, document| {
                provider.with_document(document)
            })
    }
}

pub struct RigCoreOpenAiExecutor {
    host: Arc<dyn HostProvider + Send + Sync>,
    api_key_secret_ref: String,
    default_model_id: String,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl RigCoreOpenAiExecutor {
    pub fn new(
        host: Arc<dyn HostProvider + Send + Sync>,
        api_key_secret_ref: impl Into<String>,
        default_model_id: impl Into<String>,
    ) -> KernelResult<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|_| provider_unavailable())?;
        Ok(Self {
            host,
            api_key_secret_ref: api_key_secret_ref.into(),
            default_model_id: default_model_id.into(),
            runtime: Arc::new(runtime),
        })
    }
}

impl RigBackendExecutor for RigCoreOpenAiExecutor {
    fn invoke_model(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        let secret_ref = SecretRef::new(&self.api_key_secret_ref, "Rig OpenAI API key");
        let secret = self.host.resolve_secret(secret_ref)?;
        let client = openai::Client::builder()
            .api_key(secret.expose_value())
            .build()
            .map_err(|_| provider_unavailable())?;
        let model_id = request
            .model_id
            .clone()
            .unwrap_or_else(|| self.default_model_id.clone());
        let agent = client.agent(&model_id).build();
        let prompt = request.effective_prompt_text();
        let timeout = Duration::from_millis(request.timeout_ms.unwrap_or(120_000));
        let (sender, receiver) = mpsc::sync_channel(1);
        let task = self.runtime.spawn(async move {
            let result = agent.prompt(&prompt).into_future().await.map_err(|_| ());
            let _ = sender.send(result);
        });
        let text = match receiver.recv_timeout(timeout) {
            Ok(Ok(text)) => text,
            Ok(Err(())) => return Err(provider_unavailable()),
            Err(_) => {
                task.abort();
                return Err(provider_unavailable());
            }
        };

        Ok(
            ModelResponse::text(request.model_request_id, ids::MODEL_PROVIDER_ID, text)
                .with_finish_reason("stop"),
        )
    }
}

fn provider_unavailable() -> KernelError {
    KernelError::ProviderUnavailable {
        provider_id: ids::MODEL_PROVIDER_ID.to_string(),
    }
}
