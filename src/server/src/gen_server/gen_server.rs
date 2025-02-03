use std::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet};
use wg_2024::packet::PacketType::MsgFragment;
use host::{ClientCommand, Network, Functionality, Host, SendingCodes, Message, generate_unique_session_id};
use host::ServerTypes;
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
    fn create_message(&mut self,data:Vec<u8>,destination:NodeId);
    fn servertype(&mut self,request:ClientCommand,destination:NodeId);
    fn any_string_to_message(&mut self, string:String, destination: NodeId);
}
impl GenServer {
    pub fn new(id:NodeId,
               type_host:NodeType,
               connected_drone_ids: Vec<NodeId>,
               controller_send:Sender<DroneEvent>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>, server_type: ServerTypes)->GenServer {
        let host = Host::new(id, type_host, controller_send, packet_recv, packet_send);
        GenServer{
            host,
            connected_drone_ids,
            server_type,
        }
    }
}
impl GenServeR for GenServer {
    fn create_message(&mut self, data:Vec<u8>, destination:NodeId) {
        let message=Message{
            session_id: generate_unique_session_id(),
            content: data
        };
        self.host.build_msg_fragment(destination, message).expect("TODO: panic message");
    }
    fn servertype(&mut self, request:ClientCommand, destination:NodeId){
        match request {
            ServerType => {match self.host.type_host {
                NodeType::Server=>{
                    self.any_string_to_message(match self.server_type {
                        ServerTypes::Chat => {"ServerType:Chat".to_string()}
                        ServerTypes::Content => {"ServerType:Content".to_string()}
                    }, destination)
                }
                _ => {}
            }}
            _ => {}
        }
    }
    fn any_string_to_message(&mut self, string:String, destination: NodeId){
        let mut data=string.as_bytes().to_vec();
        let tmp = data.len();
        data.insert(0, tmp as u8);
            self.create_message(data,destination);
    }
}