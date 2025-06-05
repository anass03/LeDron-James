use std::error::Error;
use std::f32;
use std::num::ParseIntError;
use std::str::FromStr;
use host::{FileId, MediaId, ServerId};
use wg_2024::network::NodeId;
use crate::backend::sc_models::LogType;
use crate::frontend::app::App;

/*
Each method here is just temporary:
if any error happens while sending commands,
it'll just print it on logs.
 */
impl App<'_>{
    // Each method here is

    /// Drone
    pub(super) fn drone_add_sender(&mut self, id:NodeId){
        match self.parse_1input(true) {
            Ok(n) => {
                match self.simulation_controller_data.add_sender(id, n as u8){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(
                        LogType::DEBUG,
                        format!("Simulation Controller Sent Add Sender to Drone ID: {:?}", id).as_str(),
                    )},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn drone_remove_sender(&mut self, id:NodeId){
        match self.parse_1input(true) {
            Ok(n) => {
                match self.simulation_controller_data.remove_sender(id, n as u8){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(
                        LogType::DEBUG,
                        format!("Simulation Controller Sent Remove Sender to Drone ID: {:?}", id).as_str(),
                    )},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn drone_set_pdr(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.set_pdr(id, n){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(
                        LogType::DEBUG,
                        format!("Simulation Controller Sent Set PDR to Drone ID: {:?}", id).as_str(),
                    )},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }


    /// Client

    pub(super) fn parse_and_send_message(&mut self, id:NodeId){
        match self.parse_3input(){
            Ok((n,s_id, str)) => {
                match self.simulation_controller_data.message_for(id, n as u8, s_id as u8, str){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(
                        LogType::DEBUG,
                        format!("Simulation Controller Sent Send Message to Client ID: {:?}", id).as_str(),
                    )},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(_) => {}
        }
    }

    pub(super) fn request_media(&mut self, id: NodeId){ // 1 Input
        // We parse the 1 input insered
        match self.parse_2input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_media(id, n.1 as ServerId, n.0 as MediaId){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Simulation Controller Sent Request Media ID {:?} to Server ID {:?}", n.0, n.1).as_str())},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn request_file(&mut self, id: NodeId){
        match self.parse_2input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_file(id, n.1 as ServerId, n.0 as FileId){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Simulation Controller Sent Request File ID {:?} to Server ID {:?}", n.0,n.1).as_str())},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn request_filelist(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_files_list(id, n as ServerId){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Simulation Controller Sent Request Files List to Server ID {:?}", n).as_str())},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            },
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn request_medialist(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_media_list(id, n as ServerId){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Simulation Controller Sent Request Media List to Server ID {:?}", n).as_str())},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            },
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn request_regtochat(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_registration_to_chat(id, n as ServerId){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Simulation Controller Sent Registration to Chat to Server ID {:?}", n).as_str())},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            },
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn request_clientlist(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_client_list(id, n as ServerId){
                    Ok(_) => {self.simulation_controller_data.logs.push_log(LogType::DEBUG, format!("Simulation Controller Sent Request Client List to Server ID {:?}", n).as_str())},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            },
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }




    fn parse_1input(&mut self, clean_after:bool)->Result<f32, String>{
        let lines = self.cache.text_buffer.inputfirst_text_area.clone().into_lines();
        match lines.get(0){
            None => {
                Err("Simulation Controller - Couldn't parse any number".to_string())
            }
            Some(str) => {
                match f32::from_str(str){
                    Ok(n) => {
                        if clean_after{
                            self.clean_topology_inputs_buffer();
                        }
                        Ok(n)
                    }
                    Err(err) => {
                        Err(err.to_string())
                    }
                }
            }
        }
    }
    fn parse_2input(&mut self, clean_after:bool)->Result<(f32, f32), String>{
        match self.parse_1input(false){
            Ok(n1) => {
                let lines = self.cache.text_buffer.inputsecond_text_area.clone().into_lines();
                match lines.get(0){
                    None => {
                        Err("Simulation Controller - Couldn't parse any number".to_string())
                    }
                    Some(str) => {
                        match f32::from_str(str){
                            Ok(n2) => {
                                if clean_after {
                                    self.clean_topology_inputs_buffer();
                                }
                                Ok((n1, n2))
                            }
                            Err(err) => {
                                Err(err.to_string())
                            }
                        }
                    }
                }
            },
            Err(str) => { return Err(str); }
        }


/*        let lines_first = self.cache.text_buffer.inputfirst_text_area.clone().into_lines();
        match lines_first.get(0){
            None => {
                Err("Simulation Controller - Couldn't parse any number".to_string())
            }
            Some(str) => {
                match f32::from_str(str){
                    Ok(n) => {
                        // we do the same from the second input text area
                        let lines_second = self
                            .cache
                            .text_buffer
                            .inputsecond_text_area
                            .clone()
                            .into_lines();

                        if lines_second.is_empty(){
                            Err("Simulation Controller - Couldn't parse any text".to_string())
                        }else{
                            // join them into a single String separated by '\n'
                            let second_string = lines_second.join("\n");
                            // clear the text areas after successful parsing
                            if clean_after{
                                self.clean_topology_inputs_buffer();
                            }
                            Ok((n, second_string))
                        }
                    }
                    Err(err) => {
                        Err(err.to_string())
                    }
                }
            }
        }*/
    }
    fn parse_3input(&mut self)->Result<(f32, f32, String), String>{
        match self.parse_2input(false){
            Ok((i1,i2)) => {
                let lines_third = self
                    .cache
                    .text_buffer
                    .inputsecond_text_area
                    .clone()
                    .into_lines();

                if lines_third.is_empty(){
                    Err("Simulation Controller - Couldn't parse any text".to_string())
                }else{
                    // join them into a single String separated by '\n'
                    let second_string = lines_third.join("\n");
                    // clear the text areas after successful parsing
                    self.clean_topology_inputs_buffer();
                    Ok((i1,i2, second_string))
                }
            }
            Err(err) => {
                Err(err)
            }
        }
    }
}