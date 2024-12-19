use std::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet};
use wg_2024::packet::PacketType::MsgFragment;
use host::{ClientToServer, HosT, SendingCodes};
use host::ServerTypes;
use host::Host;
use delegate::delegate;
use petgraph::graph::DiGraph;
use wg_2024::controller::{DroneCommand, DroneEvent};
use host::ClientToServer::ServerType;

pub struct GenServer {
    pub host:Host,
    pub server_type:ServerTypes,
}
pub trait GenServeR{
    fn create_message(&self,n_packets:usize,vtmp:&[u8],data:Vec<u8>,destination:NodeId);
    fn servertype<M>(&self,request:ClientToServer<M>,destination:NodeId);
    fn any_string_to_message(&self, string:String, destination: NodeId);
}
impl GenServer{
    pub fn new(id:NodeId,
               type_host:NodeType,
               controller_send:Sender<DroneEvent>,
               controller_recv:Receiver<DroneCommand>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>,
               connected_drone_ids:Vec<NodeId>,
               server_type:ServerTypes)->GenServer{
        GenServer{
            host: Host::new(id,
                            type_host,
                            controller_send,
                            controller_recv,
                            packet_recv,
                            packet_send,
                            connected_drone_ids,
            ),
            server_type,
        }
    }
}
impl GenServeR for GenServer{
    delegate!{
        to self.host{
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
        }
    }
    fn create_message(&self,n_packets:usize,vtmp:&[u8],data:Vec<u8>,destination:NodeId){
        let session_id= rand::random();
        let mut vf:Vec<Packet> = Vec::new();
        vf.push(Packet{
            routing_header: SourceRoutingHeader{
                hop_index: 0,
                hops: self.find_best_route(destination),
            },
            session_id ,
            pack_type: MsgFragment(Fragment{
                fragment_index :0,
                total_n_fragments : n_packets as u64,
                length:1,
                data: <[u8; 128]>::try_from(vtmp).unwrap(),
            })
        });
        let mut j=0;
        for i in 1..n_packets{
            let slice=&data[j..j+128];
            vf.push(Packet{
                routing_header: SourceRoutingHeader{
                    hop_index: 0,
                    hops: self.find_best_route(destination),
                },
                session_id,
                pack_type: (MsgFragment(Fragment{
                    fragment_index: i as u64,
                    total_n_fragments: n_packets as u64,
                    length: slice.len() as u8,
                    data: <[u8; 128]>::try_from(slice).unwrap()
                })),
            });
            j+=128;
        } // print before having the routing algorithm
        for i in vf.iter(){
            println!("{:#?}",i);
        } // sending with the rounting
        for i in vf.into_iter(){
            self.send_packet(i, None);
        }
    }
    fn servertype<M>(&self,request:ClientToServer<M>,destination:NodeId){
        match request==ServerType {
            true => {match self.host.type_host {
                NodeType::Server=>{
                    self.any_string_to_message(match self.server_type {
                        ServerTypes::Chat => {"Chat".to_string()}
                        ServerTypes::Content => {"Content".to_string()}
                    }, destination)
                }
                _ => {}
            }}
            false => {
            }
        }
    }
    fn any_string_to_message(&self,string:String,destination: NodeId){
        let mut vtmp = [0;128];
        let n_packets= string.len()/128+1;
        let mut data=string.as_bytes().to_vec();
        vtmp[..1].copy_from_slice(&[data.len() as u8]);
        data=string.as_bytes().to_vec();
        if n_packets>=1{
            self.create_message(n_packets, &vtmp,data,destination);
        }
    }
}