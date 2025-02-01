use crate::{GenServeR, GenServer};
use crossbeam_channel::{Receiver, Sender};
use delegate::delegate;
use host::{ClientCommand, Functionality, Message, SendingCodes, ServerTypes};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType};

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
            fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>;
            fn handle_ack(&self, ack: Ack, session_id: u64);
            fn handle_nack(&self, nack: Nack, session_id: u64);
            fn build_packet_ack(&self,packet: Packet,fragment_index: Option<u64>) -> Packet;
        }
        to self.info {
            fn create_message(&self,n_packets:usize,vtmp:&[u8],data:Vec<u8>,destination:NodeId,source:NodeId);
            fn any_string_to_message(&self,string:String,destination: NodeId);
            fn servertype(&self,request:ClientCommand,destination:NodeId);
        }
        to self.info.host.network{
            fn update_graph(&mut self);
            // fn modify_pdr(&mut self, drone_id: NodeId, pdr: f64);   dA fare
            fn find_best_route(&self,start:NodeId, destination: NodeId) -> Vec<NodeId>;
        }
    }
    pub fn new(id:NodeId,
           type_host:NodeType,
           connected_drone_ids:Vec<NodeId>,
           controller_send:Sender<DroneEvent>,
           controller_recv:Receiver<ClientCommand>,
           packet_recv:Receiver<Packet>,
           packet_send:HashMap<NodeId, Sender<Packet>>,
    ) ->Self{
        Self{
            info: GenServer::new(id,
                                 type_host,
                                 connected_drone_ids,
                                 controller_send,
                                 controller_recv,
                                 packet_recv,
                                 packet_send,
                                 ServerTypes::Chat
            ),
            client_list: vec![],
        }
    }
    fn run(&mut self) {
        self.log("Starting Chat Server...");
        loop {      // Listen for packets and commands
            crossbeam_channel::select_biased! { // Prioritizing Controller messages
                // using select_biased!
                // macro.
                recv(self.info.host.controller_recv) -> command => {
                    if let Ok(command) = command {
                        // self.handle_command(command);
                    }else{
                        println!("ChatServer ID {:?} - Failed to parse command-packet [Crossbeam Error - Channel Closed / Empty]", self.info.host.id);
                    }
                }
                recv(self.info.host.packet_recv) -> packet => {
                    self.handling_packets(packet);
                thread::sleep(Duration::from_millis(2000));
                // default => {
                //     println!("LeDron ID {:?} - Waiting for messages...", self.id)
                // }
                }
            }
        }
    }
    fn handling_packets(&mut self, packet:Result<Packet, crossbeam_channel::RecvError>){
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
                        self.checking_fragments(fragment, packet.routing_header.hops[0] as NodeId);
                    }
                }
            }else{
                println!("Failed to parse packet [Crossbeam Error]");
            }
        }
    }
    fn checking_fragments(&mut self, fragment: Fragment, mut destination:NodeId){
        if fragment.fragment_index == 1 && 1== fragment.total_n_fragments{
            let input = String::from_utf8_lossy(&fragment.data).to_string();
            match input.as_str() {
                "ServerType" => self.servertype(ClientCommand::ServerType, destination),
                s if s.starts_with("RegistrationToChat:") => {
                    let client_id = s.strip_prefix("RegistrationToChat:").unwrap().parse::<NodeId>().unwrap();
                    self.register_to_chat(client_id);
                }
                "ClientList" => self.send_list(destination),
                s if s.starts_with("MessageFor:") => {
                    let parts: Vec<&str> = s.strip_prefix("MessageFor:").unwrap().splitn(2, ':').collect();
                    let source=destination.clone();
                    match parts[0].parse::<u8>() {
                        Ok(number) => destination= number,
                        Err(e) => println!("Failed to parse: {:?}", e),
                    }
                    let message_data = parts[1..].join(" ");// Simulating Message<M> parsing
                    let mut session_id = None;
                    let mut content = None;
                    let mut key_value = message_data.splitn(2, ':');
                    let key = key_value.next().unwrap();
                    let value = key_value.next().unwrap();
                    match key {
                        "session_id" => session_id = Some(value.parse::<u64>().map_err(|e| e.to_string()).unwrap()),
                        "content" => content = Some(value.parse().map_err(|e| format!("{:?}", e)).unwrap()),
                        _ => self.any_string_to_message(format!("Unknown key: {}", key),destination),
                    }
                    let message = Message {
                        session_id: session_id.ok_or("Invalid format").unwrap(),
                        content: content.ok_or("Invalid format").unwrap(),
                    };
                    self.message_for_client(destination,message,source);
                }
                _ => {self.any_string_to_message("ErrNotFound".to_string(),destination);},
            }
        }else{

        }// message too long
    }
    fn message_for_client(&mut self,destination:NodeId,message:Message,source:NodeId){
        for i in self.client_list.iter(){
            if source==*i{
                let mut data=format!("MessageFrom:{} {} {}",source,message.session_id,message.content);
                let mut n_packets=data.as_bytes().to_vec().len()/128+1;
                let mut vtmp = [0;128];
                vtmp[0]='c' as u8;
                vtmp[1..2].copy_from_slice(&[data.len() as u8]);
                if n_packets>=1{
                    self.create_message(n_packets, &vtmp,data.as_bytes().to_vec(),destination,self.info.host.id);
                }
                break;
            }else{
                // Err you need to register before
            }
        }
    }
    fn register_to_chat(&mut self, id:NodeId){
        self.client_list.push(id);
    }
    fn send_list(&mut self,destination:NodeId){
        let all= self.client_list.join(" ");
        let mut data=all.as_bytes().to_vec();
        let mut n_packets=all.as_bytes().to_vec().len()/128+1;
        let mut vtmp = [0;128];
        vtmp[..1].copy_from_slice(&[data.len() as u8]);
        if n_packets>=1{
            self.create_message(n_packets, &vtmp,data,destination,self.info.host.id);
        }
    }
}