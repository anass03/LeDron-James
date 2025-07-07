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
    pub dropped_messages: Vec<u64>, // We will show max n gauges
    pub dropped_cache: usize,
    pub sent_messages: Vec<u64>,
    pub sent_cache: usize,
    pub dropped_packets_chart_width: usize,
}

impl ChartsData {
    fn default()->Self{
        Self{
            dropped_messages:Vec::new(),
            dropped_cache:0,
            sent_messages:Vec::new(),
            sent_cache:0,
            dropped_packets_chart_width:0
        }
    }
    pub(crate) fn update_dropped_per_tick(&mut self, current_dropped: usize, width: usize){
        let diff = if current_dropped != self.dropped_cache {
            let difference = current_dropped.saturating_sub(self.dropped_cache) as u64;
            self.dropped_cache = current_dropped;
            difference
        } else {
            0 // Add zero when no change to keep chart moving
        };

        self.dropped_messages.push(diff);
        if self.dropped_messages.len() > width { // Smaller fixed size
            self.dropped_messages.remove(0);
        }

    }

    pub(crate) fn update_sent_per_tick(&mut self, current_sent: usize, width: usize){
        let diff = if current_sent != self.sent_cache {
            let difference = current_sent.saturating_sub(self.sent_cache) as u64;
            self.sent_cache = current_sent;
            difference
        } else {
            0 // Add zero when no change to keep chart moving
        };

        self.sent_messages.push(diff);
        if self.sent_messages.len() > width {
            self.sent_messages.remove(0);
        }
    }
}