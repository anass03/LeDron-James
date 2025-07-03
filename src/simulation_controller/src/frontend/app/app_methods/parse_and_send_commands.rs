use std::f32;
use std::str::FromStr;
use host::{ClientId, FileId, MediaId, ServerId};
use wg_2024::network::NodeId;
use crate::backend::sc_models::LogType;
use crate::frontend::app::App;
use log::*;

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
                    Ok(_) => {},
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
                    Ok(_) => {},
                    Err(str) => {error!(target: "main", "{}", str)}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }
    pub(super) fn drone_set_pdr(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.set_pdr(id, n){
                    Ok(_) => {},
                    Err(str) => {error!(target: "main", "{}", str)}
                }
            }
            Err(str) => {error!(target: "main", "{}", str)}
        }
    }


    /// Client

    pub(super) fn parse_and_send_message_old(&mut self, id:NodeId){
        match self.parse_3input(){
            Ok((n,s_id, str)) => {
                match self.simulation_controller_data.message_for(id, n as u8, s_id as u8, str){
                    Ok(_) => {},
                    Err(str) => {error!(target: "main", "{}", str)}
                }
            }
            Err(_) => {}
        }
    }
    pub(super) fn parse_and_send_message(&mut self, client_sender:ClientId, client_receiver:ClientId, server_receiver:ServerId ){
        let chat_lines = self
            .cache
            .text_buffer
            .chat_text_area
            .clone()
            .into_lines();
        if chat_lines.is_empty(){
            // I can set an error here, like red highlighting the box
            self.simulation_controller_data.logs.push_log(LogType::ERROR, "Empty buffer, can't send message!");
        }else{
            // join them into a single String separated by '\n'
            let second_string = chat_lines.join("\n");
            // clear the text areas after successful parsing
            self.clean_topology_inputs_buffer();
            match self.simulation_controller_data.message_for(client_sender, client_receiver, server_receiver, second_string){
                Ok(_) => {
                    info!(target: "main", "Successfully sent message");
                },
                Err(str) => {
                    error!(target: "main", "{}", str);
                }
            }
        }
    }


   /* pub(super) fn request_media(&mut self, id: NodeId){ // 1 Input
        // We parse the 1 input insered
        match self.parse_2input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_media(id, n.1 as ServerId, n.0 as MediaId){
                    Ok(_) => {},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }*/
    /*pub(super) fn request_file(&mut self, id: NodeId){
        match self.parse_2input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_file(id, n.1 as ServerId, n.0 as FileId){
                    Ok(_) => {},
                    Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
                }
            }
            Err(str) => {self.simulation_controller_data.logs.push_log(LogType::ERROR, str.as_str())}
        }
    }*/
    pub(super) fn request_file_list(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_files_list(id, n as ServerId){
                    Ok(_) => {},
                    Err(str) => {error!(target: "main", "{}", str)}
                }
            },
            Err(str) => {error!(target: "main", "{}", str)}
        }
    }
    pub(super) fn request_media_list(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_media_list(id, n as ServerId){
                    Ok(_) => {},
                    Err(str) => {error!(target: "main", "{}", str)}
                }
            },
            Err(str) => {error!(target: "main", "{}", str)}
        }
    }
    pub(super) fn request_reg_to_chat(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_registration_to_chat(id, n as ServerId){
                    Ok(_) => {},
                    Err(str) => {error!(target: "main", "{}", str)}
                }
            },
            Err(str) => {error!(target: "main", "{}", str)}
        }
    }
    pub(super) fn request_client_list(&mut self, id:NodeId){
        match self.parse_1input(true){
            Ok(n) => {
                match self.simulation_controller_data.request_client_list(id, n as ServerId){
                    Ok(_) => {},
                    Err(str) => {error!(target: "main", "{}", str)}
                }
            },
            Err(str) => {error!(target: "main", "{}", str)}
        }
    }




    fn parse_1input(&mut self, clean_after:bool)->Result<f32, String>{
        let lines = self.cache.text_buffer.inputfirst_text_area.clone().into_lines();
        match lines.get(0){
            None => {
                Err("[Simulation Controller] - Couldn't parse any number".to_string())
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
                        Err("[Simulation Controller] - Couldn't parse any number".to_string())
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
            Err(str) => { Err(str) }
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
                    .inputthird_text_area
                    .clone()
                    .into_lines();

                if lines_third.is_empty(){
                    Err("[Simulation Controller] - Couldn't parse any text".to_string())
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