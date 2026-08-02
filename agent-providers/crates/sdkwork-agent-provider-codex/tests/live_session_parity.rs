use std::collections::HashSet;

use sdkwork_agent_provider_codex::{
    CodexSdkIntegration, CodexSortDirection, ThreadListCwdFilter, ThreadListParams,
    ThreadTurnsListParams, TurnItemsView,
};
use sdkwork_agent_provider_core::normalize_provider_session_path;

const LIVE_CWD_ENV: &str = "SDKWORK_LIVE_PROVIDER_SESSION_CWD";
const MAX_LIVE_ITEMS: usize = 10_000;

#[tokio::test]
#[ignore = "requires the locally installed Codex app-server and session inventory"]
async fn live_app_server_lists_project_threads_and_correlated_history() {
    let cwd = std::env::var(LIVE_CWD_ENV)
        .unwrap_or_else(|_| panic!("{LIVE_CWD_ENV} must identify the project to verify"));
    let integration = CodexSdkIntegration::bootstrap().expect("bootstrap Codex app-server");
    eprintln!("codex_live_phase=bootstrap_complete");
    let expected_cwd = normalize_provider_session_path(&cwd);
    let mut sessions = Vec::new();
    let mut session_ids = HashSet::new();
    let mut list_cursors = HashSet::new();
    let mut cursor = None;
    let mut list_page_index = 0_usize;

    loop {
        eprintln!("codex_live_phase=list_request page={list_page_index}");
        let page = integration
            .list_provider_sessions(ThreadListParams {
                cursor,
                limit: Some(sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE as u32),
                sort_key: None,
                sort_direction: None,
                model_providers: None,
                source_kinds: None,
                archived: None,
                section_id: None,
                cwd: Some(ThreadListCwdFilter::One(cwd.clone())),
                use_state_db_only: false,
                search_term: None,
                parent_thread_id: None,
                ancestor_thread_id: None,
            })
            .await
            .expect("list live Codex threads");
        eprintln!(
            "codex_live_phase=list_response page={list_page_index} items={}",
            page.data.len()
        );
        list_page_index += 1;
        for record in page.data {
            assert_eq!(
                record
                    .session
                    .cwd
                    .as_deref()
                    .map(normalize_provider_session_path)
                    .as_deref(),
                Some(expected_cwd.as_str()),
                "app-server cwd filter returned a thread from another project"
            );
            assert!(
                session_ids.insert(record.session.provider_session_id.clone()),
                "app-server repeated a thread identity across pages"
            );
            sessions.push(record.session);
        }
        assert!(
            sessions.len() <= MAX_LIVE_ITEMS,
            "live inventory is unbounded"
        );
        let Some(next_cursor) = page.next_cursor.filter(|value| !value.trim().is_empty()) else {
            break;
        };
        assert!(
            list_cursors.insert(next_cursor.clone()),
            "app-server repeated an opaque thread cursor"
        );
        cursor = Some(next_cursor);
    }

    assert!(
        !sessions.is_empty(),
        "local Codex app-server returned no threads for {cwd}"
    );

    let mut history_message_count = 0;
    let mut history_part_kinds = HashSet::new();
    for session in sessions.iter().take(10) {
        eprintln!(
            "codex_live_phase=history_session session={}",
            session.provider_session_id
        );
        let mut message_ids = HashSet::new();
        let mut history_cursors = HashSet::new();
        let mut cursor = None;
        let mut history_page_index = 0_usize;
        loop {
            eprintln!(
                "codex_live_phase=history_request page={history_page_index}"
            );
            let page = integration
                .get_provider_session_history(ThreadTurnsListParams {
                    thread_id: session.provider_session_id.clone(),
                    cursor,
                    limit: Some(sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE as u32),
                    sort_direction: Some(CodexSortDirection::Asc),
                    items_view: Some(TurnItemsView::Full),
                })
                .await
                .expect("list live Codex thread items");
            eprintln!(
                "codex_live_phase=history_response page={history_page_index} items={}",
                page.data.len()
            );
            history_page_index += 1;
            for record in page.data {
                assert_eq!(
                    record.message.provider_session_id, session.provider_session_id,
                    "app-server history crossed thread identity"
                );
                assert!(
                    message_ids.insert(record.message.provider_message_id.clone()),
                    "app-server repeated a message identity across pages"
                );
                history_message_count += 1;
                history_part_kinds.extend(record.message.parts.into_iter().map(|part| part.kind));
            }
            assert!(
                history_message_count <= MAX_LIVE_ITEMS,
                "live transcript is unbounded"
            );
            let Some(next_cursor) = page.next_cursor.filter(|value| !value.trim().is_empty())
            else {
                break;
            };
            assert!(
                history_cursors.insert(next_cursor.clone()),
                "app-server repeated an opaque history cursor"
            );
            cursor = Some(next_cursor);
        }
        if !message_ids.is_empty() {
            break;
        }
    }

    assert!(
        history_message_count > 0,
        "the newest ten local Codex threads did not contain any messages"
    );
    println!(
        "codex_live_session_parity sessions={} history_messages={} part_kinds={:?}",
        sessions.len(),
        history_message_count,
        history_part_kinds
    );
}
