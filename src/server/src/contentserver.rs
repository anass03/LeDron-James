use std::collections::HashMap;
use std::{fs, thread};
use std::fmt::{Debug, Display, Formatter};
use std::process::Command;
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::DroneEvent::{PacketDropped, PacketSent};
use wg_2024::drone;
use wg_2024::network::NodeId;
use wg_2024::packet::{Fragment, Packet, PacketType};
use wg_2024::packet::PacketType::MsgFragment;
use crate::MediaServer;
use crate::textserver::TextServer;
struct COntentServer{
    controller_recv: Receiver<Command>,
    controller_send: Sender<Command>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    rom:HashMap<u64, (u8,Vec<u8>)>,
    files:Vec<String>,
    media:Vec<String>,
    id: String
}
pub trait ContentServer:MediaServer+ TextServer{
    fn run(&mut self);
    fn handling_packets(&mut self, packet: Result<Packet,crossbeam_channel::RecvError>);
    fn create_message(&self, id:String);
    fn count_files(&self);
    fn count_media(&self,folder_path:Vec<String>);
    fn send_packet(&self, packet: Packet, channel: Option<&mut Sender<Packet>>) -> SendingCodes;
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>;
}

impl MediaServer for COntentServer {}

impl TextServer for COntentServer {}

impl ContentServer for COntentServer {
    fn run(&mut self){
        loop {      // Listen for packets and commands
            crossbeam_channel::select_biased! { // Prioritizing Controller messages using select_biased! macro.
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.handle_command(command);
                    }else{
                        // println!("Drone ID {:?} - Failed to parse command-packet [Crossbeam Error - Channel Closed / Empty]", self.id);
                    }
                }
                recv(self.packet_recv) -> packet => {
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
    fn handling_packets(&mut self, packet:Result<Packet, crossbeam_channel::RecvError>) {
        if !self.packet_recv.is_empty(){
            if let Ok(packet) = packet {
                // Only Ack, Nack, FloodResponse already sent
                // before the drone-crash will be forwarded
                // during a crashing status.
                match packet.clone().pack_type{
                    PacketType::Nack(_) |
                    PacketType::Ack(_) |
                    PacketType::FloodResponse(_) => {
                        self.handle_packet(packet);
                    },
                    // This the case of Flood Request / Fragment message:
                    PacketType::FloodRequest(_) => {

                    },
                    PacketType::MsgFragment(fragment) => {
                        self.creating_files(fragment,packet.session_id);
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
        self.media=self.media(r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\images");
        self.files=self.media(r"C:\Users\leooo\RustroverProjects\LeDron-James\src\server\files");
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
    fn send_packet(&self, packet: Packet, channel: Option<&mut Sender<Packet>>) -> SendingCodes { // OK
        self.log("Sending packet...");
        //
        match channel {
            Some(ch) => {
                match ch.send(packet.clone()) {
                    Ok(_) => {
                        // self.sendto_controller(packet, false); Ack to Sim. Controller
                        self.log("Successfully sent packet...");
                        SendingCodes::SuccessfullySent(packet.session_id)
                    },
                    Err(er) => {
                        println!("Server ID {:?} - {:?}", self.id, er.to_string());
                        let _ = self.sendto_controller(packet, false); // We send the packet to Sim.Controller
                        SendingCodes::ErrorSending(er.to_string())
                    }
                }
            },
            None => {
                let channeltosend = self.packet_send.get(&packet.clone().routing_header.hops[packet.routing_header.hop_index]);
                match channeltosend {
                    Some(ch) => {
                        match ch.send(packet.clone()) {
                            Ok(_) => {
                                self.log("Successfully sent packet...");
                                let _ = self.sendto_controller(packet.clone(), true); //Ack to Sim. Controller
                                SendingCodes::SuccessfullySent(packet.session_id)
                            },
                            Err(er) => {
                                println!("Server ID {:?} - {:?}", self.id, er.to_string());
                                let _ = self.sendto_controller(packet, true); // We send the packet to Sim.Controller
                                SendingCodes::ErrorSending(er.to_string())
                            }
                        }
                    },
                    None => {
                        self.log("Neighbour not found...");
                        let _ = self.sendto_controller(packet, true); // We send the packet to Sim.Controller
                        SendingCodes::NoNextHop("Neighbour not found".to_string())
                    }
                }
            }
        }
    }
    fn sendto_controller(&self, packet: Packet, sent: bool) -> Result<(),String>{
        self.log("Sending to controller...");
        if sent {
            match self.controller_send.send(PacketSent(packet)){
                Ok(_) => {
                    self.log("Successfully sent to controller.");
                    Ok(())
                },
                Err(er) => {
                    println!("Server ID {} - {:?}", self.id, er.to_string());
                    Err(er.to_string())
                }
            }
        }else{
            match self.controller_send.send(PacketDropped(packet)) {
                Ok(_) => {
                    self.log("Successfully sent to controller.");
                    Ok(())
                },
                Err(er) => {
                    println!("Server ID {} - {:?}", self.id, er.to_string());
                    Err(er.to_string())
                }
            }
        }
    }
}
pub enum SendingCodes { // For performance and logic enhancements
    SuccessfullySent(u64),
    ErrorSending(String),
    NoNextHop(String)
}
impl Debug for SendingCodes {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            SendingCodes::SuccessfullySent(n) => write!(f, "Successfully sent [Packet {}]", n),
            SendingCodes::ErrorSending(s) => write!(f, "Error sending the packet [{}]", s),
            SendingCodes::NoNextHop(s) => write!(f, "No next hop available [{}]", s)
        }
    }
}
impl Display for SendingCodes {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}