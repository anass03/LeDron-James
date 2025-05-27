use std::collections::HashMap;
use crossbeam_channel::unbounded;
use simulation_controller::backend::simulation_controller::{log, LogType, Logs, SimulationControllerData};
use initializer::initializer::initialize;

fn main() -> std::io::Result<()> {
    let mut thread_handles = HashMap::new();
    let mut simulation_controller_data = SimulationControllerData {
        drones: vec![],
        clients: vec![],
        servers: vec![],
        drone_command_sender: Default::default(),
        client_command_sender: Default::default(),
        server_command_sender: Default::default(),
        drone_event_receiver: unbounded().1,
        host_event_receiver: unbounded().1,
        packet_sender: Default::default(),
        log_receiver: unbounded().1,
        logs: Logs { logs_memory: vec![], log_scroll_offset: 0 },
    };
    match initialize("/Volumes/SSD FILIPPO/UNI/Advanced Programming/Drone Project/LeDron-James/init_file.toml") {
        Ok((simulation_controller, handles)) => {
            simulation_controller_data = simulation_controller;
            thread_handles = handles;
        },
        Err(error) => {log(format!("{}", error).as_str());}
    }

    let mut terminal = ratatui::init();
    let mut app = simulation_controller::frontend::app::App::default(simulation_controller_data);
    let app_result = app.run(&mut terminal);
    terminal.clear()?;
    ratatui::restore();

    for (node_id, handle) in thread_handles {
        handle.join().unwrap();
        log(&format!("Closing thread of node: {:?}", node_id));
    }

    app_result
}
