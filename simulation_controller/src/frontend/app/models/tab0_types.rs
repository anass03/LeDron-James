/// Tab n.00 data
pub(in crate::frontend::app) struct Tab0{
    pub selected_chart: usize,
    pub charts_data: ChartsData,
}

// TAB IMPLS
/// Tab0 Impl fns
impl Tab0 {
    pub(crate) fn default() -> Tab0 {
        Tab0 {
            charts_data:ChartsData::default(),
            selected_chart:0,
        }
    }
    fn next_chart(&mut self){
        if self.selected_chart != 2{
            self.selected_chart+=1;
        }
    }
    fn prev_chart(&mut self){
        if self.selected_chart != 0{
            self.selected_chart -= 1;
        }
    }
}

pub(in crate::frontend::app) struct ChartsData{ // Each CONST sec data, NOT TOTAL
    pub droppedmessages: Vec<u64>, // We will show max n gauges
    pub sentmessages: Vec<u64>,
}

impl ChartsData {
    fn default()->Self{
        Self{
            droppedmessages:Vec::new(),
            sentmessages:Vec::new(),
        }
    }
    fn plusdroppedmessages(){}
    fn plussentmessages(){}
    fn reset(){}
}