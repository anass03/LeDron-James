use crate::gen_server::gen_server::{GenServer, GenServeR};
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use host::{ClientCommand, Functionality, Message, SendingCodes, ServerTypes};
use std::collections::HashMap;
use std::str::FromStr;
use std::fmt::Debug;
use std::thread;
use std::time::Duration;
use image::EncodableLayout;
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};

pub struct ChatServer{
    info:GenServer,
    client_list:Vec<NodeId>,
}
impl ChatServer{
    delegate!{
        to self.info.host {
            fn log<S: AsRef<str>>(&self, message: S);
            fn initiate_flood(&mut self);
            fn process_flood_request(&mut self, flood_request: FloodRequest,packet: Packet);
            fn process_flood_response(&mut self, response: FloodResponse);
            fn send_packet(&self, packet: Packet, channel: Option<& Sender<Packet>>) -> SendingCodes;
            fn sendto_controller(&self, packet: Packet) -> Result<(),String>;
            fn handle_ack(&self, ack: Ack, session_id: u64);
            fn handle_nack(&mut self, nack: Nack,nack_generator:NodeId, session_id: u64);
            fn build_packet_ack(&self,packet: Packet,fragment_index: Option<u64>) -> Packet;
        }
        to self.info {
            fn create_message(&mut self,data:Vec<u8>,destination:NodeId);
            fn any_string_to_message(&mut self,string:String,destination: NodeId);
            fn servertype(&mut self,request:ClientCommand,destination:NodeId);
            fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
        }
        to self.info.host.network{
            fn update_graph(&mut self);
            fn find_best_route(&self,start:NodeId, destination: NodeId) -> Vec<NodeId>;
        }
    }
    pub fn new(id:NodeId,
               type_host:NodeType,
               connected_drone_ids:Vec<NodeId>,
               controller_send:Sender<DroneEvent>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>,
    ) ->Self{
        Self{
            info: GenServer::new(id,
                                 type_host,
                                 connected_drone_ids,
                                 controller_send,
                                 packet_recv,
                                 packet_send,
                                 ServerTypes::Chat
            ),
            client_list: vec![],
        }
    }
    pub fn run(&mut self) {
        self.log("Starting Chat Server...");
        loop {      // Listen for packets and commands
            crossbeam_channel::select_biased! { // Prioritizing Controller messages
                // using select_biased!
                // macro.
                recv(self.info.host.packet_recv) -> packet => {
                    match self.handling_packets(packet){
                        Ok(_) => (),
                        Err(_) => {
                            return;
                        }
                    };
                //thread::sleep(Duration::from_millis(2000));
                // default => {
                //     println!("LeDron ID {:?} - Waiting for messages...", self.id)
                // }
                }
            }
        }
    }
    fn handling_packets(&mut self, packet:Result<Packet, crossbeam_channel::RecvError>)->Result<(),()>{
        if !self.info.host.packet_recv.is_empty(){
            if let Ok(packet) = packet {
                // Only Ack, Nack, FloodResponse already sent
                // before the drone-crash will be forwarded
                // during a crashing status.
                match packet.clone().pack_type{
                    PacketType::Nack(_) |
                    PacketType::Ack(_) =>{}
                    PacketType::FloodResponse(e) => {
                        self.process_flood_response(e);
                    },
                    // This the case of Flood Request / Fragment message:
                    PacketType::FloodRequest(e) => {
                        self.process_flood_request(e,packet);
                    },
                    PacketType::MsgFragment(fragment) => {
                        if let Some(message) = self.handle_msg_fragment(packet.clone(),fragment){
                            if let Ok(msg_str) = String::from_utf8(message.clone()) {
                                self.checking_fragments(msg_str,packet.routing_header.hops[0]);
                            }
                        }else {
                            self.log("Failed to reassemble message fragment."); // ancora non ricevuto tutti i frammenti di quella sessione
                        }
                    }

                }
            }else{
                println!("Failed to parse packet [Crossbeam Error]");
                return Err(())
            }
        }
        Ok(())
    }
    fn checking_fragments(&mut self, input:String, mut destination:NodeId){
        match input.as_str() {
            "ServerType?" => self.servertype(ClientCommand::ServerType, destination),
            s if s.starts_with("RegistrationToChat id:") => {
                let client_id = s.strip_prefix("RegistrationToChat id:").unwrap().parse::<NodeId>().unwrap();
                self.register_to_chat(client_id);
            }
            "ClientList?" => self.send_list(destination),
            s if s.starts_with("MessageFor id:") => {
                let parts= s.strip_prefix("MessageFor id:").unwrap();
                let parts=parts.split(',').collect::<Vec<&str>>();
                let source = destination.clone();
                // match parts[0].parse::<u8>() {
                //     Ok(number) => destination = number,
                //     Err(e) => println!("Failed to parse: {:?}", e),
                // }
                if let Ok(number)= parts[0].parse::<u8>(){
                    let destination=number;
                    let content= parts[1].strip_prefix("message:").unwrap().to_string();
                    self.message_for_client(destination, content, source);
                }else{
                    println!("Failed to parse");
                }
            }
            _ => { self.any_string_to_message("ErrNotFound".to_string(), destination); },
        }
    }
    fn message_for_client(&mut self,destination:NodeId,message:String,source:NodeId){
        for i in self.client_list.iter(){
            if source==*i{
                let data=self.message_for_client_helper(message,source);
                self.create_message(data,destination);
                break;
            }else{
                self.register_to_chat(source);
                let data1= "Client was not registered but now it is and the message went through".to_string();
                self.any_string_to_message(data1,destination);
                let data=self.message_for_client_helper(message,source);
                self.create_message(data,destination);
                break;
                // Err you need to register before
            }
        }
    }
    fn message_for_client_helper(&mut self,message:String,source:NodeId)->Vec<u8>{
        let mut data=format!("MessageFrom: id:{}, message:{:?}",source,message);
        let mut data =data.as_bytes().to_vec();
        let tmp=data.len();
        data.insert(0,'c' as u8);
        data.insert(1, tmp as u8);
        data
    }
    fn register_to_chat(&mut self, id:NodeId){
        self.client_list.push(id);
    }
    fn send_list(&mut self,destination:NodeId){
        let mut data=self.client_list.clone();
        let tmp=data.len();
        data.insert(0,tmp as u8);
        self.create_message(data,destination);
    }
}