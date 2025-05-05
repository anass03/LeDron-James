use std::io;
use crate::frontend::app;

#[test]
fn test()-> io::Result<()>{

    let mut terminal = ratatui::init();
    let mut app = app::App::default();
    let app_result = app.run(&mut terminal);
    terminal.clear()?;
    ratatui::restore();
    app_result
}
