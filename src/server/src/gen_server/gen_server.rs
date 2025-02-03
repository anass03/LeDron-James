use std::collections::HashMap;
use std::time::Instant;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::{NodeId};
use wg_2024::packet::{ Fragment, NodeType, Packet};
use host::{ClientCommand, Functionality, Host, Message, generate_unique_session_id, RecSessionData};
use host::ServerTypes;
use wg_2024::controller::{DroneEvent};
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
    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>>;
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
    fn handle_msg_fragment(&mut self, packet: Packet, fragment: Fragment) -> Option<Vec<u8>> {
        // Ottieni o inizializza il buffer di frammenti per questa sessione
        let mut received_fragments = self.host.received_fragments.lock().unwrap();
        let session_data = received_fragments
            .entry(packet.session_id.clone())
            .or_insert_with(|| RecSessionData {
                buffer: vec![None; fragment.total_n_fragments as usize],
                last_activity: Instant::now(),
            });
        let buffer = &mut session_data.buffer;
        session_data.last_activity = Instant::now(); // Aggiorna il timestamp

        let index = fragment.fragment_index as usize;

        // Verifica l'indice del frammento
        if index >= buffer.len() {
            self.host.log(format!(
                "Invalid fragment index: {} (session ID: {})",
                fragment.fragment_index, packet.session_id
            ));
            return None;
        }

        // Controlla se il frammento è duplicato
        if buffer[index].is_some() {
            self.host.log(format!(
                "Duplicate fragment received: {} (session ID: {})",
                fragment.fragment_index, packet.session_id
            ));
            return None;
        }

        // Memorizza il frammento
        buffer[index] = Some(fragment.clone());

        // Invia un ACK per il frammento ricevuto
        let ack_packet = self.host.build_packet_ack(packet.clone(), Some(fragment.fragment_index));
        self.host.send_packet(ack_packet, None);

        // Verifica se tutti i frammenti sono stati ricevuti
        if buffer.iter().all(|f| f.is_some()) {
            self.host.log(format!(
                "All fragments received for session ID: {}",
                packet.session_id
            ));

            // Ricostruisce il messaggio
            let content = buffer
                .iter()
                .filter_map(|f| f.as_ref())
                .flat_map(|frag| frag.data[..frag.length as usize].to_vec())
                .collect::<Vec<u8>>();

            // Rimuove il buffer per questa sessione
            received_fragments.remove(&packet.session_id);
            return Some(content)
        }

        None
    }
}