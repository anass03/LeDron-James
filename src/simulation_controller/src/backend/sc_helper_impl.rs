use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use crossbeam_channel::{unbounded, Receiver, Sender};
use host::{ClientId, FileId, MediaId, ServerId};
use wg_2024::network::NodeId;
use crate::backend::sc_models::{LogType, SimulationHelper, WorkerCommand, WorkerResponse};
use crate::backend::simulation_controller::SimulationControllerData;

/// Worker thread helper - handles heavy operations
struct WorkerHelper {
    simulation_data: Arc<Mutex<SimulationControllerData>>,
    command_receiver: Receiver<WorkerCommand>,
    response_sender: Sender<WorkerResponse>,
    log_sender: Sender<(LogType, String)>,
    is_running: bool,
}

impl WorkerHelper {
    fn new(
        data: Arc<Mutex<SimulationControllerData>>,
        command_receiver: Receiver<WorkerCommand>,
        response_sender: Sender<WorkerResponse>,
        log_sender: Sender<(LogType, String)>,
    ) -> Self {
        Self {
            simulation_data: data,
            command_receiver,
            response_sender,
            log_sender,
            is_running: true,
        }
    }

    fn log(&self, log_type: LogType, message: String) {
        let _= self.log_sender.send((log_type, format!("[Worker] - {message}")));
    }

    fn run(mut self) {
        const TICK_DURATION: Duration = Duration::from_millis(10);

        while self.is_running {
            let tick_start = Instant::now();

            // Process commands
            self.process_commands();

            // Process_events();
            self.process_events();

            // Sleep
            let elapsed = tick_start.elapsed();
            if elapsed > TICK_DURATION {
                thread::sleep(TICK_DURATION - elapsed);
            }
        }

        // Shutdown
        self.log(LogType::INFO, "Stopping...".to_string());
        let _ = self.response_sender.send(WorkerResponse::Success);
    }

    fn process_events(&mut self) {
        match self.simulation_data.lock() {
            Ok(mut simulation_data) => {
                simulation_data.handle_channels();
            }
            Err(e) => {
                self.log(LogType::ERROR, format!("Error locking simulation_data: {}", e));
            }
        }
    }

    fn process_commands(&mut self) {
        while let Ok(command) = self.command_receiver.try_recv() {
            if let WorkerCommand::Shutdown = command {
                self.is_running = false;
                let _ = self.response_sender.send(WorkerResponse::Success);
                return;
            }

            let response = self.handle_command(command);

            if let Err(e) = self.response_sender.send(response) {
                self.log(LogType::ERROR, format!("Error sending response: {:?}", e));
                break;
            }
        }
    }

    fn handle_command(&mut self, command: WorkerCommand) -> WorkerResponse {
        match self.simulation_data.lock() {
            Ok(mut data) => {
                match command {
                    WorkerCommand::CrashDrone(drone_id) => {
                        match data.crash(drone_id) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e)
                        }
                    }
                    WorkerCommand::SetDronePDR(drone_id, pdr) => {
                        match data.set_pdr(drone_id, pdr) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::AddConnection(first_node, second_node) => {
                        match data.add_sender(first_node, second_node) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::RemoveConnection(first_node, second_node) => {
                        match data.remove_sender(first_node, second_node) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::RequestServerType(client_id) => {
                        match data.request_server_type(client_id) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::RequestFilesList(client_id, server_id) => {
                        match data.request_files_list(client_id, server_id) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::RequestFile(client_id, server_id, file_id) => {
                        match data.request_file(client_id, server_id, file_id) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::RequestMediaList(client_id, server_id) => {
                        match data.request_media_list(client_id, server_id) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::RequestMedia(client_id, server_id, media_id) => {
                        match data.request_media(client_id, server_id, media_id) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::RequestRegistrationToChat(client_id, server_id) => {
                        match data.request_registration_to_chat(client_id, server_id) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::RequestClientList(client_id, server_id) => {
                        match data.request_client_list(client_id, server_id) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::MessageFor(sender_id, receiver_id, server_id, message) => {
                        match data.message_for(sender_id, receiver_id, server_id, message) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::MediaFor(sender_id, receiver_id, server_id, path) => {
                        match data.media_for(sender_id, receiver_id, server_id, path) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::FileFor(sender_id, receiver_id, server_id, path) => {
                        match data.file_for(sender_id, receiver_id, server_id, path) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::NewMedia(sender_id, server_id, path) => {
                        match data.new_media(sender_id, server_id, path) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::NewFile(sender_id, server_id, path) => {
                        match data.new_file(sender_id, server_id, path) {
                            Ok(_) => WorkerResponse::Success,
                            Err(e) => WorkerResponse::Error(e),
                        }
                    }
                    WorkerCommand::Execute => {
                        data.handle_channels();
                        WorkerResponse::Processed
                    }
                    WorkerCommand::Shutdown => {
                        self.is_running = false;
                        WorkerResponse::Success
                    }
                }
            }
            Err(_) => {
                WorkerResponse::Error("Failed to lock data".to_string())
            }
        }
    }
}

impl SimulationHelper {
    pub fn new(controller_data: SimulationControllerData) -> Self {
        let simulation_data = Arc::new(Mutex::new(controller_data));
        let (wc_sender, wc_receiver) = unbounded();
        let (wr_sender, wr_receiver) = unbounded();

        let (log_sender, log_receiver) = unbounded();

        let worker_data = Arc::clone(&simulation_data);

        let handle = thread::spawn(move || {
            WorkerHelper::new(worker_data, wc_receiver, wr_sender, log_sender).run()
        });

        Self {
            simulation_data,
            worker_command_sender: wc_sender,
            worker_response_receiver: wr_receiver,
            worker_handle: Some(handle),
            log_receiver,
            is_running: true
        }
    }

    /// Process logs from worker (need to call this in loop)
    pub fn process_worker_logs(&self) {
        while let Ok((log_type, message)) = self.log_receiver.try_recv() {
            if let Ok(mut data) = self.simulation_data.lock() {
                data.logs.push_log(log_type, &message);
            }
        }
    }
    pub fn send_worker_command(&self, command: WorkerCommand) -> Result<(), String> {
        if !self.is_running {
            return Err(String::from("Not Running"));
        }
        self.worker_command_sender.send(command).map_err(|e| format!("Error sending command: {:?}", e))
    }

    pub fn crash(&self, drone_id: NodeId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::CrashDrone(drone_id))
    }

    pub fn set_pdr(&mut self, drone_id: NodeId, new_pdr: f32) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::SetDronePDR(drone_id, new_pdr))
    }

    pub fn add_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::AddConnection(sender_id, node_id))
    }

    pub fn remove_sender(&mut self, node_id: NodeId, sender_id: NodeId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::RemoveConnection(sender_id, node_id))
    }

    pub fn request_server_type(&mut self, client_id: ClientId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::RequestServerType(client_id))
    }

    pub fn request_files_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::RequestFilesList(client_id, server_id))
    }

    pub fn request_file(&mut self, client_id: ClientId, server_id: ServerId, file_id: FileId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::RequestFile(client_id, server_id, file_id))
    }

    pub fn request_media_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::RequestMediaList(client_id, server_id))
    }

    pub fn request_media(&mut self, client_id: ClientId, server_id: ServerId, media_id: MediaId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::RequestMedia(client_id, server_id, media_id))
    }

    pub fn request_registration_to_chat(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::RequestRegistrationToChat(client_id, server_id))
    }

    pub fn request_client_list(&mut self, client_id: ClientId, server_id: ServerId) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::RequestClientList(client_id, server_id))
    }

    pub fn message_for(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, message: String) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::MessageFor(sender_client_id, receiver_client_id, server_id, message))
    }

    pub fn media_for<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::MediaFor(sender_client_id, receiver_client_id, server_id, path))
    }

    pub fn file_for<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, receiver_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::FileFor(sender_client_id, receiver_client_id, server_id, path))
    }

    pub fn new_media<P: AsRef<Path>>(&mut self,sender_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::NewMedia(sender_client_id, server_id, path))
    }

    pub fn new_file<P: AsRef<Path>>(&mut self, sender_client_id: ClientId, server_id: ServerId, path: P) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::NewFile(sender_client_id, server_id, path))
    }

    pub fn try_receive_worker_response(&self) -> Option<WorkerResponse> {
        match self.worker_response_receiver.try_recv() {
            Ok(response) => Some(response),
            Err(_) => None,
        }
    }

    /// Send command and wait for response
    pub fn send_and_wait(&self, command: WorkerCommand, timeout: Duration) -> Result<WorkerResponse, String> {
        let start = Instant::now();
        self.send_worker_command(command)?;

        loop {
            if let Some(response) = self.try_receive_worker_response() {
                return Ok(response);
            }

            if start.elapsed() >= timeout {
                return Err("Timeout waiting for response".to_string());
            }

            thread::sleep(Duration::from_millis(1));
        }
    }

    /// Access to simulation data (thread-safe)
    pub fn with_data<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&SimulationControllerData) -> R,
    {
        match self.simulation_data.lock() {
            Ok(data) => Ok(f(&*data)),
            Err(_) => Err("Failed to lock simulation data".to_string()),
        }
    }

    /// Mutable access to simulation data (thread-safe)
    pub fn with_data_mut<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut SimulationControllerData) -> R,
    {
        match self.simulation_data.lock() {
            Ok(mut data) => Ok(f(&mut *data)),
            Err(_) => Err("Failed to lock simulation data".to_string()),
        }
    }

    /// Trigger event processing (needs to be called in loop from frontend)
    pub fn process_events(&self) -> Result<(), String> {
        self.send_worker_command(WorkerCommand::Execute)
    }

    /// Shutdown the worker
    pub fn shutdown(mut self) -> Result<(), String> {
        if self.is_running {
            self.send_worker_command(WorkerCommand::Shutdown)?;

            if let Some(handle) = self.worker_handle.take() {
                handle.join().map_err(|_| "Failed to join worker thread")?;
            }

            self.is_running = false;
        }
        Ok(())
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}