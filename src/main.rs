use crossbeam_channel::unbounded;
use tracing_subscriber::EnvFilter;
use simulation_controller::backend::simulation_controller::{log, SimulationControllerData};
use initializer::initializer::initialize;

fn main() /*-> std::io::Result<()>*/ {

    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target (true)
        .with_level(true)
        .try_init();

    let init = initialize().unwrap();
    let mut temp_sc = init.0;
    let handles = init.1;

    /*let temp_sc = SimulationControllerData {
        drones: vec![],
        clients: vec![],
        servers: vec![],
        drone_command_sender: Default::default(),
        client_command_sender: Default::default(),
        server_command_sender: Default::default(),
        drone_event_receiver: unbounded().1,
        host_event_receiver: unbounded().1,
        packet_sender: Default::default(),
    };*/

    /*let mut terminal = ratatui::init();
    let mut app = simulation_controller::frontend::app::App::default(temp_sc);
    let app_result = app.run(&mut terminal);
    terminal.clear()?;
    ratatui::restore();*/

    for (node_id, handle) in handles {
        handle.join().unwrap();
        log(&format!("Closing thread of node: {:?}", node_id));
    }

    //app_result
}
