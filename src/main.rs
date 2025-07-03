use std::collections::HashMap;
use crossbeam_channel::unbounded;
use simulation_controller::backend::simulation_controller::SimulationControllerData;
use simulation_controller::backend::sc_models::{ChatCache, LogType, Logs, NetCache, Statistics};
use initializer::initializer::initialize;
use simulation_controller::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;
use tui_logger::*;
use log::*;

fn main() -> std::io::Result<()> {
    // 1) inizializza tui-logger
    _ = init_logger(LevelFilter::Trace);
    _ = set_default_level(LevelFilter::Trace);

    // 2) (opzionale) file di log su disco
    let mut path = std::env::temp_dir();
    path.push("myapp.log");
    let file_opts = TuiLoggerFile::new(path.to_str().unwrap())
        .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        .output_file(false)
        .output_separator(':');
    set_log_file(file_opts);

    debug!(target:"main", "Logger inizializzato");

    let mut thread_handles = HashMap::new();
    let mut simulation_controller_data = SimulationControllerData {
        drones: vec![],
        clients: vec![],
        servers: vec![],
        drone_types: Default::default(),
        running_network: HashMap::new(),
        drone_command_sender: Default::default(),
        client_command_sender: Default::default(),
        server_command_sender: Default::default(),
        drone_event_receiver: unbounded().1,
        host_event_receiver: unbounded().1,
        packet_sender: Default::default(),
        //log_receiver: unbounded().1,
        logs: Logs::default(),
        stats: Statistics::default(),
        topology_algorithm_data: TopologyAlgorithm::default(),
        chat_cache: ChatCache::default(),
        net_cache: NetCache::default(),

        tx_to_renderer: None,
        rx_from_renderer: None,
    };
    match initialize("topologies/personalized_network.toml") {
        Ok((simulation_controller, handles)) => {
            simulation_controller_data = simulation_controller;
            thread_handles = handles;
        },
        Err(error) => {simulation_controller_data.logs.push_log(LogType::FATAL, format!("[Simulation Controller] - Failed to initialize: {}", error).as_str());}
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
