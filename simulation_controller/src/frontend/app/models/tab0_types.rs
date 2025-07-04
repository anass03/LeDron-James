use crate::frontend::app::App;

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
    pub dropped_cache: usize,
    pub sentmessages: Vec<u64>,
    pub dropped_packets_chart_width: usize,
}

impl ChartsData {
    fn default()->Self{
        Self{
            droppedmessages:Vec::new(),
            dropped_cache:0,
            sentmessages:Vec::new(),
            dropped_packets_chart_width:0
        }
    }
    pub(crate) fn update_dropped_per_tick(&mut self, current_dropped: usize, width: usize){
        /*let diff = current_dropped.saturating_sub(self.dropped_cache) as u64;
        self.dropped_cache = current_dropped;
        self.droppedmessages.push(diff);
        if self.droppedmessages.len() > 100 {
            self.droppedmessages.remove(0); // keep fixed size
        }*/
        let diff = if current_dropped != self.dropped_cache {
            let difference = current_dropped.saturating_sub(self.dropped_cache) as u64;
            self.dropped_cache = current_dropped;
            difference
        } else {
            0 // Add zero when no change to keep chart moving
        };

        self.droppedmessages.push(diff);
        if self.droppedmessages.len() > width { // Smaller fixed size
            self.droppedmessages.remove(0);
        }

    }
    fn plussentmessages(){}
    fn reset(){}
}