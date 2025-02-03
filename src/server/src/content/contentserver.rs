use std::collections::HashMap;
use std::{fs,thread};
use std::fmt::{Debug, Display};
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::DroneEvent::{PacketDropped, PacketSent};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};
use crate::{GenServeR, MediaServer};
use crate::TextServer;
use crate::GenServer;
use host::{ SendingCodes, ClientCommand, Functionality};
use delegate::delegate;
use wg_2024::controller::{ DroneEvent};
use host::ServerTypes;

pub struct ContentServer{
    info:GenServer,
    files:Vec<u8>,
    media:Vec<u8>,
}
pub trait COntentServer:MediaServer+ TextServer{
    fn handling_packets(&mut self, packet: Result<Packet,crossbeam_channel::RecvError>);
    fn create_vec_for_mess(&self, id:String,destination: NodeId);
    fn count_files(&mut self);
    fn count_media(&self,folder_path:String)->Vec<u8>;
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>;
    fn checking_fragments(&mut self, fragment: Fragment,destination: NodeId);
    fn lists(&mut self, request:ClientCommand,destination: NodeId);
    fn for_message_from_vec(&self, v:Vec<u8>,destination: NodeId);
}
impl ContentServer {
    delegate!{
        to self.info.host {
            fn log<S: AsRef<str>>(&self, message: S);
            fn initiate_flood(&mut self);
            fn process_flood_request(&mut self, flood_request: FloodRequest,packet: Packet);
            fn process_flood_response(&mut self, response: FloodResponse);
            fn send_packet(&self, packet: Packet, channel: Option<&Sender<Packet>>) -> SendingCodes;
            fn sendto_controller(&self, packet: Packet) -> Result<(),String>;
            fn handle_ack(&self, ack: Ack, session_id: u64);
            fn handle_nack(&mut self, nack: Nack,nack_generator:NodeId, session_id: u64);
            fn build_packet_ack(&self,packet: Packet,fragment_index: Option<u64>) -> Packet;
        }
        to self.info {
            fn create_message(&mut self,data:Vec<u8>,destination:NodeId);
            fn any_string_to_message(&mut self,string:String,destination: NodeId);
            fn servertype(&mut self,request:ClientCommand,destination:NodeId);
        }
        to self.info.host.network{
            fn update_graph(&mut self);
            fn find_best_route(&self,source:NodeId, destination: NodeId) -> Vec<NodeId>;
        }
    }
    pub fn new(id:NodeId,
           type_host:NodeType,
           connected_drone_ids: Vec<NodeId>,
           controller_send:Sender<DroneEvent>,
           packet_recv:Receiver<Packet>,
           packet_send:HashMap<NodeId, Sender<Packet>>)->Self{
        Self{
            info: GenServer::new(id,
                                 type_host,
                                 connected_drone_ids,
                                 controller_send,
                                 packet_recv,
                                 packet_send,
                                 ServerTypes::Content),
            files: Vec::new(),
            media: Vec::new(),
        }
    }
    pub fn run(&mut self){
        self.log("Starting Content Server...");
        loop {      // Listen for packets and commands
            crossbeam_channel::select_biased! { // Prioritizing Controller messages
                // using select_biased!
                // macro.
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

impl MediaServer for ContentServer {}

impl TextServer for ContentServer {}

impl COntentServer for ContentServer {
    fn handling_packets(&mut self, packet:Result<Packet, crossbeam_channel::RecvError>) {
        if !self.info.host.packet_recv.is_empty(){
            if let Ok(packet) = packet {
                match packet.clone().pack_type{
                    PacketType::Nack(_) |
                    PacketType::Ack(_) =>{},
                    PacketType::FloodResponse(e) => {
                        self.process_flood_response(e);
                    },
                    // This the case of Flood Request / Fragment message:
                    PacketType::FloodRequest(e) => {
                        self.process_flood_request(e,packet);
                    },
                    PacketType::MsgFragment(fragment) => {
                        // self.checking_fragments(fragment,packet.routing_header.hops[0]);
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
            }
        }
    }
    fn create_vec_for_mess(&mut self, id:String, destination: NodeId){
        let mut data=vec![];
        if self.media.contains(&(id.parse().unwrap())) {
            let file=Self::process_image(id.clone());
            data.push('m' as u8);
            data.push(file.width as u8);
            data.push(file.height as u8);
            data.extend(file.data);
        }else if self.files.contains(&id.parse().unwrap()) {
            let file=Self::convert_file(id.clone());
            let tmp= file.len();
            data.push('f' as u8);
            data.push(tmp as u8);
            data.extend(file);
        }
            self.create_message(data,destination);
    }
    fn count_files(&mut self) {
        self.media=self.count_media(r"https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/images/".parse().unwrap());
        self.files=self.count_media(r"https://raw.githubusercontent.com/leooon3/LeDron-James/tree/server/src/server/files/".parse().unwrap());
    }
    fn count_media(&self,folder_path:String) ->Vec<u8>{
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
        let mut j:Vec<u8> = Vec::new();
        for i in file_names{
            j.push(i.parse::<u8>().unwrap())
        }
        j
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
    fn checking_fragments(&mut self, input:String,destination: NodeId){
            // Match the string to enum variants
            match input.as_str() {
                "ServerType" => self.servertype(ClientCommand::ServerType,destination),
                "FilesList" => self.lists(ClientCommand::FilesList,destination),
                "MediaList" => self.lists(ClientCommand::MediaList,destination),
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
    }
    fn lists(&mut self, request: ClientCommand, destination: NodeId){
        match request {
            ClientCommand::FilesList => {self.count_files();self.for_message_from_vec(self.files.clone(),destination)}
            ClientCommand::MediaList => {self.count_files();self.for_message_from_vec(self.media.clone(),destination)}
            _ =>{}
        }
    }
    fn for_message_from_vec(&mut self, v: Vec<u8>, destination: NodeId){
        let all = v.iter()
            .map(|&byte| byte.to_string()) // Convert each `u8` to a string
            .collect::<Vec<String>>() // Collect into a Vec<String>
            .join(" ");
        let mut data=all.as_bytes().to_vec();
        let tmp=data.len();
        data.insert(0,tmp as u8);
        self.create_message(data,destination);
    }
}
#[test]
fn main(){

}