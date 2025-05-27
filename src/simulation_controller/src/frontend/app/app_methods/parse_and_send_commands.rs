use std::error::Error;
use std::num::ParseIntError;
use std::str::FromStr;
use wg_2024::network::NodeId;
use crate::backend::simulation_controller::LogType;
use crate::frontend::app::App;

/*
Each method here is just temporary:
if any error happens while sending commands,
it'll just print it on logs.
 */
impl App<'_>{
    // Each method here is is
    pub(super) fn parse_and_send_message(&mut self){

    }
    pub(super) fn request_media(&mut self, id: NodeId){ // 1 Input
        // We parse the 1 input insered
        match self.parse_1input(){
            Ok(n) => {
                match self.simulation_controller_data.request_media(id, n){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Simulation Controller Sent Request Media ID {n}").as_str())},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn request_file(&mut self, id: NodeId){
        match self.parse_1input(){
            Ok(n) => {
                match self.simulation_controller_data.request_file(id, n){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Simulation Controller Sent Request File ID {n}").as_str())},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    fn parse_1input(&mut self)->Result<usize, String>{
        let lines = self.cache.text_buffer.inputfirst_text_area.clone().into_lines();
        match lines.get(0){
            None => {
                Err("Simulation Controller - Couldn't parse any number".to_string())
            }
            Some(str) => {
                match usize::from_str(str){
                    Ok(n) => {
                        self.clean_topology_inputs_buffer();
                        Ok(n)
                    }
                    Err(err) => {
                        Err(err.to_string())
                    }
                }
            }
        }
    }
}