use std::collections::HashMap;
use std::{fs, thread};
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use petgraph::graph::{DiGraph};
use wg_2024::controller::DroneEvent::{PacketDropped, PacketSent};
use wg_2024::network::NodeId;
use wg_2024::packet::{FloodRequest, FloodResponse, Fragment, Packet, PacketType};
use wg_2024::packet::PacketType::MsgFragment;
use crate::MediaServer;
use crate::textserver::TextServer;
use host::{Host, HosT, SendingCodes};
use delegate::delegate;

struct COntentServer{
    host:Host,
    rom:HashMap<u64, (u8,Vec<u8>)>,
    files:Vec<String>,
    media:Vec<String>,
}
pub trait ContentServer:MediaServer+ TextServer{
    fn handling_packets(&mut self, packet: Result<Packet,crossbeam_channel::RecvError>);
    fn create_message(&self, id:String);
    fn count_files(&mut self);
    fn count_media(&self,folder_path:String)->Vec<String>;
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>;
}

impl MediaServer for COntentServer {}

impl TextServer for COntentServer {}

impl COntentServer{
    delegate!{
        to self.host {
                fn find_best_route(&self, destination: NodeId) -> Vec<NodeId>;
                fn process_flood_response(&mut self, response: FloodResponse);
                fn process_flood_request(&mut self, flood_request: FloodRequest,packet:Packet);
                fn initiate_flood(&mut self);
                fn create_graph(&mut self)->DiGraph<u8,f64>;
                fn update_graph(&mut self);
                fn log<S: AsRef<str>>(&self, message:S);
                fn modify_pdr(&mut self, drone_id: NodeId, pdr:f64);
                fn send_packet(&self, packet: Packet, channel: Option<&mut Sender<Packet>>) -> SendingCodes;
        }
    }
    fn run(&mut self){
        self.log("Starting Content Server...");
        loop {      // Listen for packets and commands
            crossbeam_channel::select_biased! { // Prioritizing Controller messages using select_biased! macro.
                recv(self.host.controller_recv) -> command => {
                    if let Ok(command) = command {
                        // self.handle_command(command);
                    }else{
                        // println!("Drone ID {:?} - Failed to parse command-packet [Crossbeam Error - Channel Closed / Empty]", self.id);
                    }
                }
                recv(self.host.packet_recv) -> packet => {
                    self.handling_packets(packet);
                thread::sleep(Duration::from_millis(2000));
                /*
                default => {
                    println!("LeDron ID {:?} - Waiting for messages...", self.id)
                }
                 */
                }
            }
        }
    }
}
impl ContentServer for COntentServer {
    fn handling_packets(&mut self, packet:Result<Packet, crossbeam_channel::RecvError>) {
        if !self.host.packet_recv.is_empty(){
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
                        // self.creating_files(fragment,packet.session_id);
                    }
                }
            }else{
                println!("Failed to parse packet [Crossbeam Error]");
            }
        }
    }
    fn create_message(&self,id:String){
        let img=Self::process_image(id.clone());
        let n_packets= img.data.len()/128+1;
        let mut vtmp:[u8;128]= [0;128];
        vtmp[..2].copy_from_slice(&[img.width as u8, img.height as u8]);

        
        let session_id= rand::random();
        let mut vf:Vec<Packet> = Vec::new();
        vf.push(Packet{
            routing_header: Default::default(),
            session_id ,
            pack_type: MsgFragment(Fragment{
                fragment_index :0,
                total_n_fragments : n_packets as u64,
                length:1,
                data: vtmp,
            })
        });
        let mut j=0;
        for i in 1..n_packets{
            let slice=&img.data[j..j+128];
            vf.push(Packet{
                routing_header: Default::default(),
                session_id,
                pack_type: (MsgFragment(Fragment{
                    fragment_index: i as u64,
                    total_n_fragments: n_packets as u64,
                    length: slice.len() as u8,
                    data: <[u8; 128]>::try_from(slice).unwrap()
                })),
            });
            j+=128;
        }
        // print before having the routing algorithm
        for i in vf.iter(){
            println!("{:#?}",i);
        }




        // sending with the rounting

        // for i in vf.into_iter(){
        //     self.send_packet(i, None);
        // }
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
        // self.log("Sending to controller...");
        if sent {
            match self.host.controller_send.send(PacketSent(packet)){
                Ok(_) => {
                    // self.log("Successfully sent to controller.");
                    Ok(())
                },
                Err(er) => {
                    println!("Server ID {} - {:?}", self.host.id, er.to_string());
                    Err(er.to_string())
                }
            }
        }else{
            match self.host.controller_send.send(PacketDropped(packet)) {
                Ok(_) => {
                    // self.log("dropped Successfully sent to controller.");
                    Ok(())
                },
                Err(er) => {
                    println!("Server ID {} - {:?}", self.host.id, er.to_string());
                    Err(er.to_string())
                }
            }
        }
    }
}