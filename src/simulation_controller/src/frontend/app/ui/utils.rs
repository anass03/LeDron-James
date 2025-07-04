// For extra functions general purpose used to compute UI modules

pub(super) mod chart_rendering{
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style, Stylize};
    use ratatui::symbols;
    use ratatui::widgets::{Axis, Block, Chart, Dataset, Sparkline, Widget};
    use crate::frontend::app::models::tab0_types::ChartsData;

    // Chart showing messages dropped each n secs.
    pub(crate) fn render_sparkline_1(chdata:&ChartsData,mut block:Block,area:Rect, buf:&mut Buffer){
        Sparkline::default()
            .style(Style::default().fg(Color::White))
            .block(block)
            .data(&chdata.sentmessages)
            .render(area,buf);
    }

    // Network Reliability
    pub(crate) fn render_chart_2(chdata:&Vec<(f64,f64)>, chdata2: Option<&Vec<(f64,f64)>>, theme: u8 ,area:Rect, buf:&mut Buffer){
        let datasets = vec![
            Dataset::default()
                .name("data2")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Cyan))
                .data(chdata.as_slice()),
        ];
        let chart = Chart::new(datasets)
            .block(Block::bordered())
            .x_axis(
                Axis::default()
                    .title("X Axis")
                    .style(Style::default().fg(Color::Gray))
            )
            .y_axis(
                Axis::default()
                    .title("Y Axis")
                    .style(Style::default().fg(Color::Gray))
                    .labels(["-20".bold(), "0".into(), "20".bold()])
                    .bounds([-20.0, 20.0]),
            );
    }

    // Distribuzione dei tipi di pacchetti che girano nella network,
    // con opzione più specifica per vedere la distribuzione dei singoli
    // tipi di NACK

    //BARCHART
    pub(crate) fn render_sparkline_3(chdata:&ChartsData, area:Rect, buf:&mut Buffer){

    }
}

