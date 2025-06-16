// For extra functions general purpose used to compute UI modules

pub(super) mod chart_rendering{
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Sparkline, Widget};
    use crate::frontend::app::models::tab0_types::ChartsData;

    // Chart showing messages dropped each n secs.
    pub(crate) fn render_sparkline_1(chdata:&ChartsData,mut block:Block,area:Rect, buf:&mut Buffer){
        block = block.title("Dropped Messages");
        let sparkline = Sparkline::default()
            .style(Style::default().fg(Color::White))
            .block(block)
            .data(&chdata.sentmessages)
            .render(area,buf);
    }

    // Network Reliability
    pub(crate) fn render_sparkline_2(chdata:&ChartsData, area:Rect, buf:&mut Buffer){

    }

    // Distribuzione dei tipi di pacchetti che girano nella network,
    // con opzione più specifica per vedere la distribuzione dei singoli
    // tipi di NACK

    //BARCHART
    pub(crate) fn render_sparkline_3(chdata:&ChartsData, area:Rect, buf:&mut Buffer){

    }
}

