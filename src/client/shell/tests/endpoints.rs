use super::*;

#[path = "workspace_navigation.rs"]
mod workspace_navigation;
use crate::client::endpoint::ClientEndpointId;

fn agent(
    name: &str,
    status: crate::api::schema::AgentStatus,
    state_change_seq: u64,
) -> ClientShellAgent {
    ClientShellAgent {
        pane_id: "pane_1".into(),
        workspace_id: "ws_1".into(),
        tab_id: "tab_1".into(),
        name: Some(name.into()),
        display_agent: None,
        agent: Some("pi".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: status,
        state_change_seq,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: true,
    }
}

fn current_workspace_view() -> crate::api::schema::AgentViewSetParams {
    use crate::api::schema::{
        AgentViewBuiltinField, AgentViewContext, AgentViewField, AgentViewFilter, AgentViewValue,
    };

    crate::api::schema::AgentViewSetParams {
        source: "example.views".into(),
        label: Some("current space".into()),
        filter: Some(AgentViewFilter::Eq {
            field: AgentViewField::Builtin(AgentViewBuiltinField::WorkspaceId),
            value: AgentViewValue::Context {
                context: AgentViewContext::CurrentWorkspaceId,
            },
        }),
        sort: Vec::new(),
    }
}

#[test]
fn newer_snapshot_does_not_reuse_stale_agent_view_projection() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut local = snapshot();
    local.agent_view_label = Some("current space".into());
    state.set_snapshot(Box::new(local.clone()));
    state.set_test_endpoint_agent_view(&ClientEndpointId::Local, Some(current_workspace_view()));
    let endpoint = state
        .endpoints
        .iter()
        .find(|endpoint| endpoint.endpoint_id.is_local())
        .expect("local endpoint");
    assert!(matches!(
        ClientShellState::endpoint_agent_view(endpoint),
        Some(Ok(Some(_)))
    ));

    state.set_test_endpoint_agent_view_projection(
        &ClientEndpointId::Local,
        "foreign-boot",
        99,
        None,
    );
    let endpoint = state
        .endpoints
        .iter()
        .find(|endpoint| endpoint.endpoint_id.is_local())
        .expect("local endpoint");
    assert!(matches!(
        ClientShellState::endpoint_agent_view(endpoint),
        Some(Ok(Some(_)))
    ));

    local.revision += 1;
    state.set_snapshot(Box::new(local));
    let endpoint = state
        .endpoints
        .iter()
        .find(|endpoint| endpoint.endpoint_id.is_local())
        .expect("local endpoint");
    assert!(ClientShellState::endpoint_agent_view(endpoint).is_none());
}

#[test]
fn selected_position_sort_uses_public_tab_and_pane_numbers() {
    use crate::api::schema::{
        AgentStatus, AgentViewBuiltinSortField, AgentViewSort, AgentViewSortField,
        AgentViewSortOrder,
    };

    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut selected = snapshot();
    selected.agent_view_label = Some("positions".into());

    let mut tab_nine = selected.tabs[0].clone();
    tab_nine.tab_id = "ws_1:t9".into();
    tab_nine.number = 9;
    let mut tab_two = tab_nine.clone();
    tab_two.tab_id = "ws_1:t2".into();
    tab_two.number = 2;
    selected.tabs = vec![tab_nine, tab_two];

    let mut pane_tab_nine = selected.panes[0].clone();
    pane_tab_nine.tab_id = "ws_1:t9".into();
    pane_tab_nine.pane_id = "ws_1:p1".into();
    let mut pane_nine = pane_tab_nine.clone();
    pane_nine.tab_id = "ws_1:t2".into();
    pane_nine.pane_id = "ws_1:p9".into();
    let mut pane_two = pane_nine.clone();
    pane_two.pane_id = "ws_1:p2".into();
    selected.panes = vec![pane_tab_nine, pane_nine, pane_two];

    let mut late_tab = agent("tab nine", AgentStatus::Idle, 1);
    late_tab.tab_id = "ws_1:t9".into();
    late_tab.pane_id = "ws_1:p1".into();
    let mut late_pane = agent("pane nine", AgentStatus::Idle, 1);
    late_pane.tab_id = "ws_1:t2".into();
    late_pane.pane_id = "ws_1:p9".into();
    let mut early_pane = agent("pane two", AgentStatus::Idle, 1);
    early_pane.tab_id = "ws_1:t2".into();
    early_pane.pane_id = "ws_1:p2".into();
    selected.agents = vec![late_tab, late_pane, early_pane];
    state.set_snapshot(Box::new(selected));

    let mut view = current_workspace_view();
    view.label = Some("positions".into());
    view.filter = None;
    view.sort = vec![
        AgentViewSort {
            field: AgentViewSortField::Builtin(AgentViewBuiltinSortField::TabOrder),
            order: AgentViewSortOrder::Asc,
        },
        AgentViewSort {
            field: AgentViewSortField::Builtin(AgentViewBuiltinSortField::PaneOrder),
            order: AgentViewSortOrder::Asc,
        },
    ];
    state.set_test_endpoint_agent_view(&ClientEndpointId::Local, Some(view));

    let names = aggregate_navigation::aggregate_agent_rows(
        &state.endpoints,
        &state.active_endpoint_id,
        crate::config::AgentPanelSortConfig::Priority,
    )
    .into_iter()
    .map(|row| row.agent.name.as_deref().expect("agent name"))
    .collect::<Vec<_>>();
    assert_eq!(names, ["pane two", "pane nine", "tab nine"]);
}
