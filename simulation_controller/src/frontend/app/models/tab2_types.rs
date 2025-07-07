use crate::frontend::app::models::general_types::TOMLMessages;
use ratatui::widgets::ListState;

/// Tab n.02 data
pub(in crate::frontend::app) struct Tab2 {
    pub compiler_buf: TOMLMessages,
    pub topology_list_state: ListState,
}

/// Tab2 Impl fns
impl Tab2 {
    pub(crate) fn default() -> Self {
        let mut topology_state = ListState::default();
        topology_state.select(Some(0)); // Start with first item selected
        Self {
            compiler_buf: TOMLMessages::FirstInitialized,
            topology_list_state: topology_state,
        }
    }
}
