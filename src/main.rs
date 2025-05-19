use std::collections::HashMap;
use crossbeam_channel::unbounded;
use simulation_controller::backend::simulation_controller::{log, SimulationControllerData};
use initializer::initializer::initialize;

fn main() /*-> std::io::Result<()>*/ {
    /*for (node_id, handle) in handles {
        handle.join().unwrap();
        println!("Closing thread of node: {:?}", node_id);
    }
     */

    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .with_level(true)
        .try_init();

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
    };
    match initialize("init_file.toml") {
        Ok((simulation_controller, handles)) => {
            simulation_controller_data = simulation_controller;
            thread_handles = handles;
        },
        Err(error) => {log(format!("{}", error).as_str());}
    }

    /*match simulation_controller_data.crash(3) {
        Ok(_) => {},
        Err(error) => {log(format!("{}", error).as_str());}
    }*/

    /*let temp_sc = SimulationControllerData {
        drones: vec![],
        clients: vec![],
        servers: vec![],
        drone_command_sender: Default::default(),
        client_command_sender: Default::default(),
        server_command_sender: Default::default(),
        drone_event_receiver: unbounded().1,d
        host_event_receiver: unbounded().1,
        packet_sender: Default::default(),
    };*/

    /*let mut terminal = ratatui::init();
    let mut app = simulation_controller::frontend::app::App::default(temp_sc);
    let app_result = app.run(&mut terminal);
    terminal.clear()?;
    ratatui::restore();*/

    for (node_id, handle) in thread_handles {
        handle.join().unwrap();
        log(&format!("Closing thread of node: {:?}", node_id));
    }

    //check why hosts don't see other hosts
    //check why panic when crashing drone

    //app_result
}
