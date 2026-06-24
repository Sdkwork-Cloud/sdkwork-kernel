use sdkwork_agent_business::PromptAiRepository;
use sdkwork_intelligence_prompts_ai_contract::{
    AgentPromptTemplateKind, AgentPromptTemplateListQuery, AgentPromptTemplateRecord,
    CreatePromptBindingCommand, CreatePromptCommand, CreatePromptVersionCommand,
    ListPromptBindingsQuery, ListPromptsQuery, ListPromptVersionsQuery, PromptAiBindingItem,
    PromptAiError, PromptAiItem, PromptAiResult, PromptAiVersionItem, PromptBindingRecord,
    PromptRecord, PromptVersionRecord, PublishPromptVersionCommand, RenderPromptVersionCommand,
    UpdatePromptBindingCommand, UpdatePromptCommand,
};

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
    async fn list_prompts(&self, _query: ListPromptsQuery) -> PromptAiResult<Vec<PromptAiItem>> {
        Ok(vec![])
    }

    async fn create_prompt(&self, _command: CreatePromptCommand) -> PromptAiResult<PromptAiItem> {
        Err(PromptAiError::internal("stub"))
    }

    async fn update_prompt(&self, _command: UpdatePromptCommand) -> PromptAiResult<PromptAiItem> {
        Err(PromptAiError::internal("stub"))
    }

    async fn list_versions(
        &self,
        _query: ListPromptVersionsQuery,
    ) -> PromptAiResult<Vec<PromptAiVersionItem>> {
        Ok(vec![])
    }

    async fn create_version(
        &self,
        _command: CreatePromptVersionCommand,
    ) -> PromptAiResult<PromptAiVersionItem> {
        Err(PromptAiError::internal("stub"))
    }

    async fn publish_version(
        &self,
        _command: PublishPromptVersionCommand,
    ) -> PromptAiResult<Option<PromptAiVersionItem>> {
        Ok(None)
    }

    async fn render_version(
        &self,
        _command: RenderPromptVersionCommand,
    ) -> PromptAiResult<Option<String>> {
        Ok(None)
    }

    async fn list_bindings(
        &self,
        _query: ListPromptBindingsQuery,
    ) -> PromptAiResult<Vec<PromptAiBindingItem>> {
        Ok(vec![])
    }

    async fn create_binding(
        &self,
        _command: CreatePromptBindingCommand,
    ) -> PromptAiResult<PromptAiBindingItem> {
        Err(PromptAiError::internal("stub"))
    }

    async fn update_binding(
        &self,
        _command: UpdatePromptBindingCommand,
    ) -> PromptAiResult<Option<PromptAiBindingItem>> {
        Ok(None)
    }

    async fn get_prompt(&self, _tenant_id: i64, _id: i64) -> PromptAiResult<PromptRecord> {
        Err(PromptAiError::not_found("stub"))
    }

    async fn get_prompt_version(
        &self,
        _tenant_id: i64,
        _id: i64,
    ) -> PromptAiResult<PromptVersionRecord> {
        Err(PromptAiError::not_found("stub"))
    }

    async fn list_bindings_for_owner(
        &self,
        _tenant_id: i64,
        _organization_id: i64,
        _owner_type: &str,
        _owner_id: i64,
    ) -> PromptAiResult<Vec<PromptBindingRecord>> {
        Ok(vec![])
    }

    async fn get_agent_prompt_template(
        &self,
        _tenant_id: i64,
        _id: i64,
    ) -> PromptAiResult<AgentPromptTemplateRecord> {
        Err(PromptAiError::not_found("stub"))
    }

    async fn list_agent_prompt_templates(
        &self,
        _query: AgentPromptTemplateListQuery,
    ) -> PromptAiResult<Vec<AgentPromptTemplateRecord>> {
        Ok(vec![])
    }
}
