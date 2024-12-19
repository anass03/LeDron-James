use std::collections::HashMap;
use std::{fs, thread};
use std::fmt::{Debug, Display};
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use petgraph::graph::DiGraph;
use wg_2024::controller::DroneEvent::{PacketDropped, PacketSent};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType};
use crate::{GenServeR, MediaServer};
use crate::TextServer;
use crate::GenServer;
use host::{HosT, SendingCodes, ClientToServer};
use delegate::delegate;
use wg_2024::controller::{DroneCommand, DroneEvent};
use host::ServerTypes;

struct COntentServer{
    info:GenServer,
    files:Vec<String>,
    media:Vec<String>,
}
pub trait ContentServer:MediaServer+ TextServer{
    fn handling_packets(&mut self, packet: Result<Packet,crossbeam_channel::RecvError>);
    fn create_vec_for_mess(&self, id:String,destination: NodeId);
    fn count_files(&mut self);
    fn count_media(&self,folder_path:String)->Vec<String>;
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>;
    fn checking_fragments(&mut self, fragment: Fragment,destination: NodeId);
    fn lists<M>(&mut self, request:ClientToServer<M>,destination: NodeId);
    fn for_message_from_vec(&self, v:Vec<u8>,destination: NodeId) ->Vec<u8>;
}
impl COntentServer{
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
           connected_drone_ids:Vec<NodeId>)->Self{
        Self{
            info: GenServer::new(id,
                                 type_host,
                                 controller_send,
                                 controller_recv,
                                 packet_recv,
                                 packet_send,
                                 connected_drone_ids,
                                 ServerTypes::Content),
            files: Vec::new(),
            media: Vec::new(),
        }
    }
    fn run(&mut self){
        self.log("Starting Content Server...");
        loop {      // Listen for packets and commands
            crossbeam_channel::select_biased! { // Prioritizing Controller messages
                // using select_biased!
                // macro.
                recv(self.info.host.controller_recv) -> command => {
                    if let Ok(command) = command {
                        // self.handle_command(command);
                    }else{
                        println!("Server ID {:?} - Failed to parse command-packet [Crossbeam Error - Channel Closed / Empty]", self.info.host.id);
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
}

impl ContentServer for COntentServer {
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
                        self.checking_fragments(fragment,packet.routing_header.hops[0]);
                    }
                }
            }else{
                println!("Failed to parse packet [Crossbeam Error]");
            }
        }
    }
    fn create_vec_for_mess(&self,id:String,destination: NodeId){
        let mut n_packets=0;
        let mut vtmp = [0;128];
        let mut data=vec![];
        if self.media.contains(&id){
            let file=Self::process_image(id.clone());
            n_packets= file.data.len()/128+1;
            vtmp[..2].copy_from_slice(&[file.width as u8, file.height as u8]);
            data=file.data;
        }else if self.files.contains(&id) {
            let file=Self::convert_file(id.clone());
            n_packets= file.len()/128+1;
            vtmp[..1].copy_from_slice(&[file.len() as u8]);
            data=file;
        }
        if n_packets>=1{
            self.create_message(n_packets, &vtmp,data,destination);
        }
    }
    fn count_files(&mut self) {
        self.media=self.count_media(r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\images".parse().unwrap());
        self.files=self.count_media(r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\files".parse().unwrap());
    }
    fn count_media(&self,folder_path:String) ->Vec<String>{
        // Read the directory
        let entries = fs::read_dir(folder_path).unwrap();
        // Collect file names into a vector
        let file_names: Vec<String> = entries
            .filter_map(|entry| entry.ok()) // Filter out errors
            .filter_map(|entry| {
                let path = entry.path();
                if path.is_file() { // Check if it's a file
                    path.file_name().and_then(|name| name.to_str().map(String::from))
                } else {
                    None
                }
            })
            .collect();
        file_names
    }
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>{
        self.log("Sending to controller...");
        if sent {
            match self.info.host.controller_send.send(PacketSent(packet)){
                Ok(_) => {
                    self.log("Successfully sent to controller.");
                    Ok(())
                },
                Err(er) => {
                    println!("Server ID {} - {:?}", self.info.host.id, er.to_string());
                    Err(er.to_string())
                }
            }
        }else{
            match self.info.host.controller_send.send(PacketDropped(packet)) {
                Ok(_) => {
                    self.log("dropped Successfully sent to controller.");
                    Ok(())
                },
                Err(er) => {
                    println!("Server ID {} - {:?}", self.info.host.id, er.to_string());
                    Err(er.to_string())
                }
            }
        }
    }
    fn checking_fragments(&mut self, fragment: Fragment,destination: NodeId){
        if fragment.fragment_index == 1 && 1== fragment.total_n_fragments{
            // Convert Vec<u8> to String
            let input = String::from_utf8(Vec::from(fragment.data)).map_err(|e| e.to_string()).ok().unwrap();
            // Match the string to enum variants
            match input.as_str() {
                "ServerType" => self.servertype(ClientToServer::ServerType,destination),
                "FilesList" => self.lists(ClientToServer::FilesList,destination),
                "MediaList" => self.lists(ClientToServer::MediaList,destination),
                s if s.starts_with("File:") => {
                    let file_id = s.strip_prefix("File:").unwrap().to_string();
                    self.create_vec_for_mess(file_id,destination);
                }
                s if s.starts_with("Media:") => {
                    let media_id = s.strip_prefix("Media:").unwrap().to_string();
                    self.create_vec_for_mess(media_id,destination);
                }
                _ => { self.any_string_to_message("ErrNotFound".to_string(),destination)}
            }
        }else{}
    }
    fn lists<M>(&mut self, request:ClientToServer<M>,destination: NodeId){
        match request {
            ClientToServer::FilesList => {self.count_files();self.from_vec_to_vec(self.files.clone(),destination)}
            ClientToServer::MediaList => {self.count_files();self.from_vec_to_vec(self.media.clone(),destination)}
            _ =>{}
        }
    }
    fn for_message_from_vec(&self, v: Vec<u8>,destination: NodeId){
        let all= v.join(" ");
        let mut data=all.as_bytes().to_vec();
        let mut n_packets=all.as_bytes().to_vec().len()/128+1;
        let mut vtmp = [0;128];
        vtmp[..1].copy_from_slice(&[data.len() as u8]);
        if n_packets>=1{
            self.create_message(n_packets, &vtmp,data,destination);
        }
    }
}
impl MediaServer for COntentServer {}
impl TextServer for COntentServer {}
