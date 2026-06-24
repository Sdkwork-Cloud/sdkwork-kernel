use sdkwork_agent_business::{
    AgentBusinessService, AgentMcpAuthKind, AgentMcpServerCreateCommand,
    AgentMcpServerUpdateCommand, AgentMcpTransportKind, AgentVisibility, AllowAllPolicyProvider,
    InMemoryAgentAuditSink, InMemoryAgentRepository, PromptAiRepository,
};
use sdkwork_intelligence_prompts_ai_contract::AgentPromptTemplateKind;

fn service(
) -> AgentBusinessService<InMemoryAgentRepository, InMemoryAgentAuditSink, AllowAllPolicyProvider> {
    AgentBusinessService::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("provider.policy.market"),
    )
}

#[test]
fn prompt_templates_owned_by_sdkwork_prompts_contract() {
    fn assert_prompt_ai_repository<T: PromptAiRepository>() {}
    assert_prompt_ai_repository::<InMemoryPromptAiRepositoryStub>();
    let kind = AgentPromptTemplateKind::System;
    assert_eq!(serde_json::to_string(&kind).unwrap(), "\"system\"");
}

struct InMemoryPromptAiRepositoryStub;

#[async_trait::async_trait]
impl PromptAiRepository for InMemoryPromptAiRepositoryStub {
    async fn list_prompts(
        &self,
        _query: sdkwork_intelligence_prompts_ai_contract::ListPromptsQuery,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        Vec<sdkwork_intelligence_prompts_ai_contract::PromptAiItem>,
    > {
        Ok(vec![])
    }

    async fn create_prompt(
        &self,
        _command: sdkwork_intelligence_prompts_ai_contract::CreatePromptCommand,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        sdkwork_intelligence_prompts_ai_contract::PromptAiItem,
    > {
        Err(sdkwork_intelligence_prompts_ai_contract::PromptAiError::internal("stub"))
    }

    async fn list_versions(
        &self,
        _query: sdkwork_intelligence_prompts_ai_contract::ListPromptVersionsQuery,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        Vec<sdkwork_intelligence_prompts_ai_contract::PromptAiVersionItem>,
    > {
        Ok(vec![])
    }

    async fn create_version(
        &self,
        _command: sdkwork_intelligence_prompts_ai_contract::CreatePromptVersionCommand,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        sdkwork_intelligence_prompts_ai_contract::PromptAiVersionItem,
    > {
        Err(sdkwork_intelligence_prompts_ai_contract::PromptAiError::internal("stub"))
    }

    async fn publish_version(
        &self,
        _command: sdkwork_intelligence_prompts_ai_contract::PublishPromptVersionCommand,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        Option<sdkwork_intelligence_prompts_ai_contract::PromptAiVersionItem>,
    > {
        Ok(None)
    }

    async fn render_version(
        &self,
        _command: sdkwork_intelligence_prompts_ai_contract::RenderPromptVersionCommand,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<Option<String>> {
        Ok(None)
    }

    async fn list_bindings(
        &self,
        _query: sdkwork_intelligence_prompts_ai_contract::ListPromptBindingsQuery,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        Vec<sdkwork_intelligence_prompts_ai_contract::PromptAiBindingItem>,
    > {
        Ok(vec![])
    }

    async fn create_binding(
        &self,
        _command: sdkwork_intelligence_prompts_ai_contract::CreatePromptBindingCommand,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        sdkwork_intelligence_prompts_ai_contract::PromptAiBindingItem,
    > {
        Err(sdkwork_intelligence_prompts_ai_contract::PromptAiError::internal("stub"))
    }

    async fn update_binding(
        &self,
        _command: sdkwork_intelligence_prompts_ai_contract::UpdatePromptBindingCommand,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        Option<sdkwork_intelligence_prompts_ai_contract::PromptAiBindingItem>,
    > {
        Ok(None)
    }

    async fn get_prompt(
        &self,
        _tenant_id: i64,
        _id: i64,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        sdkwork_intelligence_prompts_ai_contract::PromptRecord,
    > {
        Err(sdkwork_intelligence_prompts_ai_contract::PromptAiError::not_found("stub"))
    }

    async fn get_prompt_version(
        &self,
        _tenant_id: i64,
        _id: i64,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        sdkwork_intelligence_prompts_ai_contract::PromptVersionRecord,
    > {
        Err(sdkwork_intelligence_prompts_ai_contract::PromptAiError::not_found("stub"))
    }

    async fn list_bindings_for_owner(
        &self,
        _tenant_id: i64,
        _organization_id: i64,
        _owner_type: &str,
        _owner_id: i64,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        Vec<sdkwork_intelligence_prompts_ai_contract::PromptBindingRecord>,
    > {
        Ok(vec![])
    }

    async fn get_agent_prompt_template(
        &self,
        _tenant_id: i64,
        _id: i64,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        sdkwork_intelligence_prompts_ai_contract::AgentPromptTemplateRecord,
    > {
        Err(sdkwork_intelligence_prompts_ai_contract::PromptAiError::not_found("stub"))
    }

    async fn list_agent_prompt_templates(
        &self,
        _query: sdkwork_intelligence_prompts_ai_contract::AgentPromptTemplateListQuery,
    ) -> sdkwork_intelligence_prompts_ai_contract::PromptAiResult<
        Vec<sdkwork_intelligence_prompts_ai_contract::AgentPromptTemplateRecord>,
    > {
        Ok(vec![])
    }
}
