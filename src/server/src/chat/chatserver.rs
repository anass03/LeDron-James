use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use host::{ClientToServer, HosT, Message, SendingCodes, ServerTypes};
use delegate::delegate;
use petgraph::graph::DiGraph;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType};
use crate::{GenServeR, GenServer};

struct ChatServer{
    info:GenServer,
    client_list:Vec<String>,
}
impl ChatServer{
    delegate!{
        to self.info {
            fn log<S: AsRef<str>>(&self, message: S);
            fn initiate_flood(&mut self);
            fn process_flood_request(&mut self, flood_request: FloodRequest,packet: Packet);
            fn process_flood_response(&mut self, response: FloodResponse);
            fn create_graph(&mut self)->DiGraph<u8,f64>;
            fn update_graph(&mut self);
            fn modify_pdr(&mut self, drone_id: NodeId, pdr: f64);
            fn find_best_route(&self, destination: NodeId) -> Vec<NodeId>;
            fn send_packet(&self, packet: Packet, channel: Option<&mut Sender<Packet>>) -> SendingCodes;
            fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>;
            fn handle_ack(&self, ack: Ack, session_id: u64);
            fn handle_nack(&self, nack: Nack, session_id: u64);
            fn build_packet_flood_response(&self,floodrequest_header: FloodRequest, search_id:u64) -> Packet;
            fn build_packet_nack(&self,packet: Packet, nack_id: NackType, optional_fragment_index: Option<u64>) -> Packet;
            fn build_packet_ack(&self,packet: Packet, ack_id: Ack) -> Packet;
            fn create_message(&self,n_packets:usize,vtmp:&[u8],data:Vec<u8>,destination:NodeId);
            fn any_string_to_message(&self,string:String,destination: NodeId);
            fn servertype<M>(&self,request:ClientToServer<M>,destination:NodeId);
        }
    }
    fn new(id:NodeId,
           type_host:NodeType,
           controller_send:Sender<DroneEvent>,
           controller_recv:Receiver<DroneCommand>,
           packet_recv:Receiver<Packet>,
           packet_send:HashMap<NodeId, Sender<Packet>>,
           connected_drone_ids:Vec<NodeId>
    )->Self{
        Self{
            info: GenServer::new(id,
                                 type_host,
                                 controller_send,
                                 controller_recv,
                                 packet_recv,
                                 packet_send,
                                 connected_drone_ids,
                                 ServerTypes::Chat
            ),
            client_list: vec![],
        }
    }
    fn run(&mut self){
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
    fn handling_packets(&mut self, packet:Result<Packet, crossbeam_channel::RecvError>) {
        if !self.info.host.packet_recv.is_empty(){
            if let Ok(packet) = packet {
                // Only Ack, Nack, FloodResponse already sent
                // before the drone-crash will be forwarded
                // during a crashing status.
                match packet.clone().pack_type{
                    PacketType::Nack(_) |
                    PacketType::Ack(_) |
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
    fn checking_fragments<M>(&mut self, fragment: Fragment, mut destination:NodeId){
        if fragment.fragment_index == 1 && 1== fragment.total_n_fragments{
            let input = String::from_utf8(Vec::from(fragment.data)).map_err(|e| e.to_string()).ok().unwrap();
            match input.as_str() {
                "ServerType" => self.servertype(ClientToServer::ServerType, destination),
                s if s.starts_with("RegistrationToChat:") => {
                    let client_id = s.strip_prefix("RegistrationToChat:").unwrap().to_string();
                    self.register_to_chat(client_id);
                }
                "ClientList" => self.send_list(destination),
                s if s.starts_with("File:") => {
                    let file_id = s.strip_prefix("File:").unwrap().to_string();
                    self.create_vec_for_mess(file_id,destination);
                }
                s if s.starts_with("MessageFor:") => {
                    let parts: Vec<&str> = s.strip_prefix("MessageFor:").unwrap().splitn(2, ':').collect();
                    destination = parts[0].to_string();
                    let message_data = parts[1..].to_string(); // Simulating Message<M> parsing
                    let mut source_id = None;
                    let mut session_id = None;
                    let mut content = None;
                    let mut key_value = message_data.splitn(2, ':');
                    let key = key_value.next().ok_or("Invalid format")?;
                    let value = key_value.next().ok_or("Invalid format")?;
                    match key {
                        "source_id" => source_id = Some(value.parse::<NodeId>().map_err(|e| e.to_string())?),
                        "session_id" => session_id = Some(value.parse::<u64>().map_err(|e| e.to_string())?),
                        "content" => content = Some(value.parse::<M>().map_err(|e| e.to_string())?),
                        _ => self.any_string_to_message(format!("Unknown key: {}", key),destination),
                    }
                    let message = Message {
                        source_id: source_id.ok_or("Invalid format")?,
                        session_id: session_id.ok_or("Invalid format")?,
                        content: content.ok_or("Invalid format")?,
                    };
                    self.message_for_client(destination,message);
                }
                _ => {self.any_string_to_message("ErrNotFound".to_string(),destination);},
            }
        }else{}// message too long
    }
    fn message_for_client<M>(&mut self,destination:NodeId,message:Message<M>){
        let mut data=format!("MessageFrom:{} {} {}",message.source_id,message.session_id,message.content);
        let mut n_packets=data.as_bytes().to_vec().len()/128+1;
        let mut vtmp = [0;128];
        vtmp[..1].copy_from_slice(&[data.len() as u8]);
        if n_packets>=1{
            self.create_message(n_packets, &vtmp,data.as_bytes().to_vec(),destination);
        }
    }
    fn register_to_chat(&mut self, id:String){
        self.client_list.push(id);
    }
    fn send_list(&mut self,destination:NodeId){
        let all= self.client_list.join(" ");
        let mut data=all.as_bytes().to_vec();
        let mut n_packets=all.as_bytes().to_vec().len()/128+1;
        let mut vtmp = [0;128];
        vtmp[..1].copy_from_slice(&[data.len() as u8]);
        if n_packets>=1{
            self.create_message(n_packets, &vtmp,data,destination);
        }
    }
}