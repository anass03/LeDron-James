// src/utils.rs
use crossbeam_channel::Receiver;
use host::NetCommand;
use tracing::info;

pub fn handle_server_control_loop(
    controller_recv: &Receiver<NetCommand>,
    mut handler: impl FnMut(NetCommand),
) {
    if let Ok(cmd) = controller_recv.try_recv() {
        match &cmd {
            NetCommand::Net(_) | NetCommand::ControllerShortcut(_) => {
                //info!("Server received control command: {:?}", cmd);
                handler(cmd);
            }
        }
    }
}
