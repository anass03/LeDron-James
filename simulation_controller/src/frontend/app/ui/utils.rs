// For extra functions general purpose used to compute UI modules

pub(super) mod chart_rendering{
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style, Stylize};
    use ratatui::symbols;
    use ratatui::widgets::{Axis, Block, Chart, Dataset, Sparkline, Widget};
    use crate::frontend::app::models::tab0_types::ChartsData;

    // Chart showing messages dropped each n secs.
    pub(crate) fn render_sparkline_1(chdata: &mut ChartsData, mut block:Block, area:Rect, buf:&mut Buffer){
        chdata.dropped_packets_chart_width = area.width as usize;
        Sparkline::default()
            .style(Style::default().fg(Color::White))
            .block(block)
            .data(&chdata.droppedmessages)
            .render(area,buf);
    }

    // Network Reliability
    pub(crate) fn render_sparkline_2(chdata: &mut ChartsData, mut block:Block, area:Rect, buf:&mut Buffer){
        Sparkline::default()
            .style(Style::default().fg(Color::White))  // Different color from dropped messages
            .block(block)
            .data(&chdata.sentmessages)
            .render(area,buf);
    }

    // Distribuzione dei tipi di pacchetti che girano nella network,
    // con opzione più specifica per vedere la distribuzione dei singoli
    // tipi di NACK

    //BARCHART
    pub(crate) fn render_sparkline_3(chdata:&ChartsData, area:Rect, buf:&mut Buffer){

    }
}

