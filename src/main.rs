use std::collections::HashMap;
use crossbeam_channel::unbounded;
use simulation_controller::backend::simulation_controller::SimulationControllerData;
use simulation_controller::backend::sc_models::{Logs, Statistics};
use initializer::initializer::initialize;
use simulation_controller::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;

fn main() -> std::io::Result<()> {
    let mut thread_handles = HashMap::new();
    let mut simulation_controller_data = SimulationControllerData {
        drones: vec![],
        clients: vec![],
        servers: vec![],
        running_network: HashMap::new(),
        drone_command_sender: Default::default(),
        client_command_sender: Default::default(),
        server_command_sender: Default::default(),
        drone_event_receiver: unbounded().1,
        host_event_receiver: unbounded().1,
        packet_sender: Default::default(),
        log_receiver: unbounded().1,
        logs: Logs::default(),
        stats: Statistics::default(),
        topology_algorithm_data: TopologyAlgorithm::default(),
    };
    match initialize("init_file.toml") {
        Ok((simulation_controller, handles)) => {
            simulation_controller_data = simulation_controller;
            thread_handles = handles;
        },
        Err(error) => {println!("[Simulation Controller] - Failed to initialize: {}", error);}
    }

    let mut terminal = ratatui::init();
    let mut app = simulation_controller::frontend::app::App::default(simulation_controller_data);
    let app_result = app.run(&mut terminal);
    terminal.clear()?;
    ratatui::restore();

    for (node_id, handle) in thread_handles {
        handle.join().unwrap();
        println!("[Simulation Controller] - Closing thread of node: {:?}", node_id);
    }

    app_result
}
