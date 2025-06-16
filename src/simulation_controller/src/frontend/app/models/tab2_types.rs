use crate::frontend::app::models::general_types::TOMLmessages;

/// Tab n.02 data
pub(in crate::frontend::app) struct Tab2{
    pub compiler_buf: TOMLmessages,
}

/// Tab2 Impl fns
impl Tab2{
    pub(crate) fn default() ->Self{
        Self{
            compiler_buf:TOMLmessages::FirstInizialized,
        }
    }
}