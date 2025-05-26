use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use crossbeam_channel::unbounded;
use host::Message;
use simulation_controller::backend::simulation_controller::{log, LogType, Logs, SimulationControllerData};
use initializer::initializer::initialize;

fn main() -> std::io::Result<()> {

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
        log_receiver: unbounded().1,
        logs: Logs { logs_memory: vec![], log_scroll_offset: 0 },
    };
    match initialize("init_file.toml") {
        Ok((simulation_controller, handles)) => {
            simulation_controller_data = simulation_controller;
            thread_handles = handles;
        },
        Err(error) => {log(format!("{}", error).as_str());}
    }

    /*sleep(Duration::from_secs(5));
    log("Requesting registration to chat for client 7");
    match simulation_controller_data.request_registration_to_chat(7) {
        Ok(_) => {},
        Err(error) => {log(format!("{}", error).as_str());}
    }

    sleep(Duration::from_secs(15));
    log("Requesting registration to chat for client 6");
    match simulation_controller_data.request_registration_to_chat(6) {
        Ok(_) => {},
        Err(error) => {log(format!("{}", error).as_str());}
    }

    sleep(Duration::from_secs(15));
    log("Requesting registration to chat for client 6");
    match simulation_controller_data.message_for(6, 7, Message { session_id: 0, content: vec![] }) {
        Ok(_) => {},
        Err(error) => {log(format!("{}", error).as_str());}
    }*/     //simulate command

    let log_receiver = simulation_controller_data.log_receiver.clone();
    let mut logs_arc = simulation_controller_data.logs.clone();
    std::thread::spawn(move || {
        for received in log_receiver.iter() {
            logs_arc.push_log(LogType::DEBUG, received.as_str());
        }
    });

    let mut terminal = ratatui::init();
    let mut app = simulation_controller::frontend::app::App::default(simulation_controller_data);
    let app_result = app.run(&mut terminal);
    terminal.clear()?;
    ratatui::restore();       //ratatui


    for (node_id, handle) in thread_handles {
        handle.join().unwrap();
        log(&format!("Closing thread of node: {:?}", node_id));
    }

    //check why panic when crashing drone

    app_result
}
