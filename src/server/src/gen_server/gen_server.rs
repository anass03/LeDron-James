use std::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet};
use wg_2024::packet::PacketType::MsgFragment;
use host::{ClientCommand, Network, Functionality, Host, SendingCodes};
use host::ServerTypes;
use host::Host::*;
use delegate::delegate;
use petgraph::graph::DiGraph;
use wg_2024::controller::{DroneCommand, DroneEvent};
use host::ClientCommand::ServerType;

pub struct GenServer{
    pub host:Host,
    pub connected_drone_ids:Vec<NodeId>,
    pub server_type:ServerTypes,
}
pub trait GenServeR{
    fn create_message(&self,n_packets:usize,vtmp:&[u8],data:Vec<u8>,destination:NodeId,source:NodeId);
    fn servertype<M: host::DroneSend>(&self,request:ClientCommand,destination:NodeId);
    fn any_string_to_message(&self, string:String, destination: NodeId);
}
impl GenServer {
    pub fn new(id:NodeId,
               type_host:NodeType,
               connected_drone_ids: Vec<NodeId>,
               controller_send:Sender<DroneEvent>,
               controller_recv:Receiver<ClientCommand>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>, server_type: ServerTypes)->GenServer {
        let host = Host::new(id, type_host, controller_send, controller_recv, packet_recv, packet_send);
        GenServer{
            host,
            connected_drone_ids,
            server_type,
        }
    }
}
impl GenServeR for GenServer {
    fn create_message(&self,n_packets:usize,vtmp:&[u8],data:Vec<u8>,destination:NodeId,source:NodeId) {
        let session_id= rand::random();
        let mut vf:Vec<Packet> = Vec::new();
        vf.push(Packet{
            routing_header: SourceRoutingHeader{
                hop_index: 0,
                hops: self.host.network.find_best_route(source,destination),
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
                    hops: self.host.network.find_best_route(source,destination),
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
            self.host.send_packet(i, None);
        }
    }
    fn servertype<M: host::DroneSend>(&self,request:ClientCommand,destination:NodeId){
        match request {
            ServerType => {match self.host.type_host {
                NodeType::Server=>{
                    self.any_string_to_message(match self.server_type {
                        ServerTypes::Chat => {"Chat".to_string()}
                        ServerTypes::Content => {"Content".to_string()}
                    }, destination)
                }
                _ => {}
            }}
            _ => {}
        }
    }
    fn any_string_to_message(&self,string:String,destination: NodeId){
        let mut vtmp = [0;128];
        let n_packets= string.len()/128+1;
        let mut data=string.as_bytes().to_vec();
        vtmp[..1].copy_from_slice(&[data.len() as u8]);
        data=string.as_bytes().to_vec();
        if n_packets>=1{
            self.create_message(n_packets, &vtmp,data,destination,self.host.id);
        }
    }
}