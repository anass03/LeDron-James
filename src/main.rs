use std::collections::HashMap;
use crossbeam_channel::unbounded;
use simulation_controller::backend::simulation_controller::SimulationControllerData;
use simulation_controller::backend::sc_models::{ChatCache, LogType, Logs, NetCache, Statistics};
use initializer::initializer::initialize;
use simulation_controller::frontend::app::topology_algorithm::alg_models::TopologyAlgorithm;
use tui_logger::*;
use log::*;

fn main() -> std::io::Result<()> {
    // 1) initialize tui-logger
    _ = init_logger(LevelFilter::max());
    _ = set_default_level(LevelFilter::max());

    // 2) (opzionale) file di log su disco
    let mut path = std::env::temp_dir();
    path.push("myapp.log");
    let file_opts = TuiLoggerFile::new(path.to_str().unwrap())
        .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        .output_file(false)
        .output_separator(':');
    set_log_file(file_opts);

    debug!(target:"main", "Initialized Logging");

    let mut thread_handles = HashMap::new();
    let mut simulation_controller_data = SimulationControllerData::default();
    match initialize("topologies/double_chain_network.toml") {
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
