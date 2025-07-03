use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use client::ClientCommand;
use crossbeam_channel::unbounded;
use host::{ClientId, FileId, HostCommand, MediaId, ServerId};
use wg_2024::controller::DroneCommand;
use wg_2024::network::NodeId;
use crate::backend::sc_models::{AsyncCommand, AsyncProcessor, AsyncSimulationController, LogType, SharedAsyncData};
use crate::backend::simulation_controller::SimulationControllerData;

impl AsyncSimulationController {
    pub fn new(simulation_data: SimulationControllerData) -> Self {
        let (tx, rx) = unbounded::<AsyncCommand>();
        let shared_data = Arc::new(Mutex::new(SharedAsyncData::default()));
        let shared_data_clone = Arc::clone(&shared_data);

        // Creating asynchronous processor
        thread::spawn(move || {
            AsyncProcessor::new(rx, shared_data_clone).run();
        });

        Self {
            data: simulation_data,
            command_sender: tx,
            shared_data,
        }
    }

    // Tick for processing continuously
    pub fn tick(&mut self) {
        // Process shared data
        if let Ok(mut shared) = self.shared_data.try_lock() {
            // Transfer logs from thread
            for (log_type, message) in shared.async_logs.drain(..) {
                self.data.logs.push_sc_log(log_type, message);
            }

            // Update stats
            if shared.failed_commands > 0 {
                self.data.logs.push_sc_log(
                    LogType::WARN,
                    format!("Async processor: {} failed commands", shared.failed_commands)
                );
                shared.failed_commands = 0;
            }
        }

        // Process events synchronously
        self.data.handle_channels();
        self.data.handle_rx_renderer();
    }

    pub fn handle_channels(&mut self) {
        self.data.handle_channels();
    }

    pub fn handle_rx_renderer(&mut self) {
        self.data.handle_rx_renderer();
    }

    pub fn crash(&self, drone_id: NodeId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::Crash(drone_id))
            .map_err(|_| "Failed to send crash command".to_string())
    }

    pub fn set_pdr(&self, drone_id: NodeId, new_pdr: f32) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::SetPDR(drone_id, new_pdr))
            .map_err(|_| "Failed to send set_pdr command".to_string())
    }

    pub fn add_sender(&self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::AddSender(node_id, sender_id))
            .map_err(|_| "Failed to send add_sender command".to_string())
    }

    pub fn remove_sender(&self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::RemoveSender(node_id, sender_id))
            .map_err(|_| "Failed to send remove_sender command".to_string())
    }

    pub fn request_server_type(&self, client_id: ClientId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::RequestServerType(client_id))
            .map_err(|_| "Failed to send request_server_type command".to_string())
    }

    pub fn request_files_list(&self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::RequestFilesList(client_id, server_id))
            .map_err(|_| "Failed to send request_files_list command".to_string())
    }

    pub fn request_file(&self, client_id: ClientId, server_id: ServerId, file_id: FileId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::RequestFile(client_id, server_id, file_id))
            .map_err(|_| "Failed to send request_file command".to_string())
    }

    pub fn request_media_list(&self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::RequestMediaList(client_id, server_id))
            .map_err(|_| "Failed to send request_media_list command".to_string())
    }

    pub fn request_media(&self, client_id: ClientId, server_id: ServerId, media_id: MediaId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::RequestMedia(client_id, server_id, media_id))
            .map_err(|_| "Failed to send request_media command".to_string())
    }

    pub fn request_registration_to_chat(&self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::RequestRegistrationToChat(client_id, server_id))
            .map_err(|_| "Failed to send request_registration_to_chat command".to_string())
    }

    pub fn request_client_list(&self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::RequestClientList(client_id, server_id))
            .map_err(|_| "Failed to send request_client_list command".to_string())
    }

    pub fn message_for(&self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, message: String) -> Result<(), String> {
        self.command_sender.send(AsyncCommand::MessageFor(sender_client_id, receiver_client_id, server_id, message))
            .map_err(|_| "Failed to send message_for command".to_string())
    }

    pub fn media_for<P: AsRef<Path>>(&self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let path_str = path.as_ref().to_str().unwrap_or("").to_string();
        self.command_sender.send(AsyncCommand::MediaFor(sender_client_id, receiver_client_id, server_id, path_str))
            .map_err(|_| "Failed to send media_for command".to_string())
    }

    pub fn file_for<P: AsRef<Path>>(&self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let path_str = path.as_ref().to_str().unwrap_or("").to_string();
        self.command_sender.send(AsyncCommand::FileFor(sender_client_id, receiver_client_id, server_id, path_str))
            .map_err(|_| "Failed to send file_for command".to_string())
    }

    pub fn new_media<P: AsRef<Path>>(&self, sender_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let path_str = path.as_ref().to_str().unwrap_or("").to_string();
        self.command_sender.send(AsyncCommand::NewMedia(sender_client_id, server_id, path_str))
            .map_err(|_| "Failed to send new_media command".to_string())
    }

    pub fn new_file<P: AsRef<Path>>(&self, sender_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        let path_str = path.as_ref().to_str().unwrap_or("").to_string();
        self.command_sender.send(AsyncCommand::NewFile(sender_client_id, server_id, path_str))
            .map_err(|_| "Failed to send new_file command".to_string())
    }
}

impl AsyncProcessor {
    fn new(receiver: crossbeam_channel::Receiver<AsyncCommand>, shared_data: Arc<Mutex<SharedAsyncData>>) -> Self {
        Self {
            receiver,
            shared_data,
            drone_command_senders: HashMap::new(),
            client_command_senders: HashMap::new(),
            server_command_senders: HashMap::new(),
        }
    }

    fn run(mut self) {
        // Processor is active
        if let Ok(mut shared) = self.shared_data.lock() {
            shared.is_processing = true;
        }

        self.log(LogType::INFO, "Async processor started".to_string());

        // Main loop
        while let Ok(command) = self.receiver.try_recv() {
            let start_time = std::time::Instant::now();

            match self.process_command(command) {
                Ok(_) => {
                    self.increment_processed();
                    let duration = start_time.elapsed();
                    if duration > Duration::from_millis(100) {
                        self.log(LogType::WARN, format!("Slow command took {}ms", duration.as_millis()));
                    }
                }
                Err(e) => {
                    self.increment_failed();
                    self.log(LogType::ERROR, format!("Command failed: {}", e));
                }
            }
        }

        self.log(LogType::INFO, "Async processor stopped".to_string());

        // Processor is not active
        if let Ok(mut shared) = self.shared_data.lock() {
            shared.is_processing = false;
        }
    }

    fn process_command(&mut self, command: AsyncCommand) -> Result<(), String> {
        match command {
            AsyncCommand::Crash(drone_id) => {
                self.log(LogType::DEBUG, format!("Processing crash for drone {}", drone_id));

                if let Some(sender) = self.drone_command_senders.get(&drone_id) {
                    sender.send(DroneCommand::Crash)
                        .map_err(|e| format!("Failed to send crash command: {}", e))?;

                    self.log(LogType::INFO, format!("Sent crash command to drone {}", drone_id));
                } else {
                    return Err(format!("Drone {} sender not found", drone_id));
                }
            }

            AsyncCommand::SetPDR(drone_id, pdr) => {
                self.log(LogType::DEBUG, format!("Setting PDR {} for drone {}", pdr, drone_id));

                if pdr < 0.0 || pdr > 1.0 {
                    return Err(format!("Invalid PDR value: {}", pdr));
                }

                if let Some(sender) = self.drone_command_senders.get(&drone_id) {
                    sender.send(DroneCommand::SetPacketDropRate(pdr))
                        .map_err(|e| format!("Failed to send PDR command: {}", e))?;

                    self.log(LogType::INFO, format!("Set PDR {} for drone {}", pdr, drone_id));
                } else {
                    return Err(format!("Drone {} sender not found", drone_id));
                }
            }

            AsyncCommand::RequestMedia(client_id, server_id, media_id) => {
                self.log(LogType::DEBUG, format!("Requesting media {} from server {} for client {}",
                                                 media_id, server_id, client_id));

                if let Some(sender) = self.client_command_senders.get(&client_id) {
                    let command = ClientCommand::Net(
                        host::ClientNetCommand::Media(server_id, media_id.clone())
                    );

                    sender.send(command)
                        .map_err(|e| format!("Failed to send media request: {}", e))?;

                    self.log(LogType::INFO, format!("Requested media {} for client {}", media_id, client_id));
                } else {
                    return Err(format!("Client {} sender not found", client_id));
                }
            }

            AsyncCommand::MessageFor(sender_id, receiver_id, server_id, message) => {
                self.log(LogType::DEBUG, format!("Sending message from {} to {} via server {}",
                                                 sender_id, receiver_id, server_id));

                if let Some(sender) = self.client_command_senders.get(&sender_id) {
                    let command = ClientCommand::Net(
                        host::ClientNetCommand::MessageFor(receiver_id, server_id, message.clone())
                    );

                    sender.send(command)
                        .map_err(|e| format!("Failed to send message: {}", e))?;

                    self.log(LogType::INFO, format!("Sent message from {} to {}", sender_id, receiver_id));
                } else {
                    return Err(format!("Client {} sender not found", sender_id));
                }
            }

            AsyncCommand::RequestServerType(client_id) => {
                self.log(LogType::DEBUG, format!("Requesting server type for client {}", client_id));

                if let Some(sender) = self.client_command_senders.get(&client_id) {
                    let command = ClientCommand::Net(host::ClientNetCommand::ServerType);

                    sender.send(command)
                        .map_err(|e| format!("Failed to send server type request: {}", e))?;

                    self.log(LogType::INFO, format!("Requested server type for client {}", client_id));
                } else {
                    return Err(format!("Client {} sender not found", client_id));
                }
            }

            AsyncCommand::RequestFilesList(client_id, server_id) => {
                if let Some(sender) = self.client_command_senders.get(&client_id) {
                    let command = ClientCommand::Net(host::ClientNetCommand::FilesList(server_id));
                    sender.send(command)
                        .map_err(|e| format!("Failed to send files list request: {}", e))?;
                    self.log(LogType::INFO, format!("Requested files list for client {}", client_id));
                } else {
                    return Err(format!("Client {} sender not found", client_id));
                }
            }

            AsyncCommand::RequestMediaList(client_id, server_id) => {
                if let Some(sender) = self.client_command_senders.get(&client_id) {
                    let command = ClientCommand::Net(host::ClientNetCommand::MediaList(server_id));
                    sender.send(command)
                        .map_err(|e| format!("Failed to send media list request: {}", e))?;
                    self.log(LogType::INFO, format!("Requested media list for client {}", client_id));
                } else {
                    return Err(format!("Client {} sender not found", client_id));
                }
            }

            AsyncCommand::RequestClientList(client_id, server_id) => {
                if let Some(sender) = self.client_command_senders.get(&client_id) {
                    let command = ClientCommand::Net(host::ClientNetCommand::ClientList(server_id));
                    sender.send(command)
                        .map_err(|e| format!("Failed to send client list request: {}", e))?;
                    self.log(LogType::INFO, format!("Requested client list for client {}", client_id));
                } else {
                    return Err(format!("Client {} sender not found", client_id));
                }
            }

            AsyncCommand::AddSender(first, second) => {
            }
            AsyncCommand::RemoveSender(_, _) => {}
            AsyncCommand::RequestFile(_, _, _) => {}
            AsyncCommand::RequestRegistrationToChat(_, _) => {}
            AsyncCommand::MediaFor(_, _, _, _) => {}
            AsyncCommand::FileFor(_, _, _, _) => {}
            AsyncCommand::NewMedia(_, _, _) => {}
            AsyncCommand::NewFile(_, _, _) => {}
        }

        Ok(())
    }

    fn log(&self, log_type: LogType, message: String) {
        if let Ok(mut shared) = self.shared_data.lock() {
            shared.async_logs.push((log_type, format!("[AsyncProcessor] - {}", message)));
        }
    }

    fn increment_processed(&self) {
        if let Ok(mut shared) = self.shared_data.lock() {
            shared.processed_commands += 1;
        }
    }

    fn increment_failed(&self) {
        if let Ok(mut shared) = self.shared_data.lock() {
            shared.failed_commands += 1;
        }
    }

    pub fn initialize_senders(&mut self,
                              drone_senders: HashMap<NodeId, crossbeam_channel::Sender<DroneCommand>>,
                              client_senders: HashMap<NodeId, crossbeam_channel::Sender<ClientCommand>>,
                              server_senders: HashMap<NodeId, crossbeam_channel::Sender<HostCommand>>) {

        self.drone_command_senders = drone_senders;
        self.client_command_senders = client_senders;
        self.server_command_senders = server_senders;

        self.log(LogType::INFO, "Initialized command senders".to_string());
    }
}