use initializer::initializer::initialize;

fn main() -> std::io::Result<()> {
    //let (mut controller, handles) = initialize();
    //controller.crash(2);
    /*for (node_id, handle) in handles {
        handle.join().unwrap();
        println!("Closing thread of node: {:?}", node_id);
    }
     */

    let mut terminal = ratatui::init();
    let mut app = simulation_controller::frontend::app::App::default();
    let app_result = app.run(&mut terminal);
    terminal.clear()?;
    ratatui::restore();
    app_result
}
