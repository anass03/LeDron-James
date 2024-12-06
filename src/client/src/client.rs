use std::any::Any;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, PacketType, FloodRequest, FloodResponse, Ack, Nack, NackType, NodeType, Fragment};
use std::collections::{HashMap, HashSet};
use std::time::Duration;


pub struct Client {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    packet_send: Sender<Packet>,
    topology: HashMap<NodeId, Vec<NodeId>>, // Memorizza la topologia
    flood_cache: HashMap<u64, bool>,       // Memorizza FloodRequest già visti
    graph: Graph, // Grafo per rappresentare la rete
    pdr_data: HashMap<NodeId, f32>, // PDR stimata per ogni nodo connesso
}

impl Client {
    pub fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: Sender<Packet>,
    ) -> Self {
        assert!(
            connected_drone_ids.len() >= 1 && connected_drone_ids.len() <= 2,
            "Client must be connected to at least one and at most two drones!"
        );
        Self {
            id,
            connected_drone_ids,
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
            topology: HashMap::new(),
            flood_cache: HashMap::new(),
            graph: Graph::new(),
            pdr_data: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        self.initiate_flood();
        loop {
            crossbeam_channel::select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.handle_command(command);
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.handle_packet(packet);
                    }
                }
                default => {
                    // Simula inattività, evita spinning continuo
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
    }

    fn handle_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::AddSender(node_id, sender) => {
                if self.connected_drone_ids.contains(&node_id) {
                    println!("Client {}: Drone {} is already connected!", self.id, node_id);
                } else if self.connected_drone_ids.len() < 2 {
                    self.connected_drone_ids.push(node_id);
                    println!("Client {}: Added connection to drone {}", self.id, node_id);
                } else {
                    println!("Client {}: Cannot connect to more than two drones!", self.id);
                }
            }

            _ => {}
        }
    }

    fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type {
            PacketType::FloodResponse(response) => {
                self.process_flood_response(response);
            }
            PacketType::MsgFragment(fragment) => {
                self.handle_fragment(fragment);
            }
            PacketType::FloodRequest(flood_request) => {
                self.process_flood_request(flood_request);
            }
            PacketType::Ack(ack) => {
                self.handle_ack(ack, packet.session_id);
            }
            PacketType::Nack(nack) => {
                self.handle_nack(nack, packet.session_id);
            }
            _ => println!("host {}: Received unsupported packet type", self.id),
        }
    }
    fn handle_ack(&self, ack: Ack, session_id: u64) {
        println!(
            "Client {}: Received Ack for fragment index {} (session ID: {})",
            self.id, ack.fragment_index, session_id
        );

        // Aggiorna stato o logica per confermare la ricezione
        // Ad esempio: rimuovere il frammento dalla lista dei frammenti da inviare
    }
    fn handle_nack(&self, nack: Nack, session_id: u64) {
        println!(
            "Client {}: Received Nack for fragment index {} (session ID: {}). Reason: {:?}",
            self.id, nack.fragment_index, session_id, nack.nack_type
        );

        match nack.nack_type {
            NackType::Dropped => {
                println!("Client {}: Retrying fragment due to drop.", self.id);
                // Logica per ritrasmettere il frammento
            }
            NackType::ErrorInRouting(node_id) => {
                println!(
                    "Client {}: Routing error encountered. Node ID: {}",
                    self.id, node_id
                );
                // Adotta azioni per correggere il percorso, se possibile
            }
            _ => {
                println!("Client {}: Unhandled Nack type: {:?}", self.id, nack.nack_type);
            }
        }
    }


    fn initiate_flood(&mut self) {
        let flood_request = Packet {
            pack_type: PacketType::FloodRequest(FloodRequest {
                flood_id: rand::random(),
                initiator_id: self.id,
                path_trace: vec![(self.id, NodeType::Client)],
            }),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: vec![self.id],
            },
            session_id: rand::random(),
        };

        if self.packet_send.send(flood_request).is_err() {
            println!("Client {}: Error initiating flood", self.id);
        }
    }

    fn process_flood_request(&mut self, flood_request: FloodRequest) {
        if self.flood_cache.contains_key(&flood_request.flood_id) {
            return; // Già processata
        }
        self.flood_cache.insert(flood_request.flood_id, true);

        println!("Client {}: Processing FloodRequest {:?}", self.id, flood_request);

        // Inoltra la richiesta di Flood a droni collegati
        for &drone_id in &self.connected_drone_ids {
            let mut request = flood_request.clone();
            request.path_trace.push((drone_id, NodeType::Drone));
            let packet = Packet {
                pack_type: PacketType::FloodRequest(request),
                routing_header: SourceRoutingHeader {
                    hop_index: 1,
                    hops: vec![self.id, drone_id],
                },
                session_id: rand::random(),
            };
            if let Err(e) = self.packet_send.send(packet) {
                println!("Client {}: Error forwarding FloodRequest: {:?}", self.id, e);
            }
        }
    }


    fn process_flood_response(&mut self, response: FloodResponse) {
        println!(
            "Client {}: Processing FloodResponse for flood ID: {}",
            self.id, response.flood_id
        );

        // Itera sui nodi nel percorso della FloodResponse
        let path = &response.path_trace;
        for i in 0..path.len() - 1 {
            let (current_node, _) = path[i];
            let (next_node, _) = path[i + 1];

            // Ottieni la PDR tra i nodi
            let pdr = self.pdr_data.get(&next_node).cloned().unwrap_or(1.0); // Default PDR 1.0

            // Aggiungi il collegamento al grafo
            self.graph.add_edge(current_node, next_node, pdr);
            self.graph.add_edge(next_node, current_node, pdr); // Per grafi bidirezionali
        }

        println!("Client {}: Updated topology: {:?}", self.id, self.graph);
    }


    fn handle_fragment(&self, fragment: Fragment) {
        println!(
            "Client {}: Received fragment index {} of total {}",
            self.id, fragment.fragment_index, fragment.total_n_fragments
        );

        // Invia un Ack per confermare la ricezione
        let ack = Packet {
            pack_type: PacketType::Ack(Ack {
                fragment_index: fragment.fragment_index,
            }),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: vec![self.id],
            },
            session_id: rand::random(),
        };

        if self.packet_send.send(ack).is_err() {
            println!("Client {}: Error sending Ack", self.id);
        }

        // Logica per ricomporre i frammenti
    }
    // fn evaluate_drones(&mut self) {
    //     for &drone_id in &self.connected_drone_ids {
    //         let mut sent = 0;
    //         let mut acked = 0;
    //
    //         // Testa con un certo numero di pacchetti
    //         for _ in 0..10 {
    //             let test_packet = Packet {
    //                 pack_type: PacketType::MsgFragment(Fragment {
    //                     fragment_index: 0,
    //                     total_n_fragments: 1,
    //                     length: 0,
    //                     data: [0; 128],
    //                 }),
    //                 routing_header: SourceRoutingHeader {
    //                     hop_index: 0,
    //                     hops: vec![self.id, drone_id],
    //                 },
    //                 session_id: rand::random(),
    //             };
    //
    //             if self.packet_send.send(test_packet).is_ok() {
    //                 sent += 1;
    //             }
    //
    //             // Verifica se riceviamo un Ack
    //             if let Ok(DroneEvent::PacketSent(_)) = self.controller_recv.recv_timeout(Duration::from_millis(100)) {
    //                 acked += 1;
    //             }
    //         }
    //
    //         // Calcola la PDR
    //         let pdr = if sent > 0 { acked as f32 / sent as f32 } else { 1.0 };
    //         self.pdr_data.insert(drone_id, pdr);
    //
    //         println!(
    //             "Client {}: Estimated PDR for drone {}: {:.2}",
    //             self.id, drone_id, pdr
    //         );
    //     }
    // }

    fn find_best_route(&self, destination: NodeId) -> Vec<NodeId> {
        let mut dist = HashMap::new();
        let mut prev = HashMap::new();
        let mut unvisited: HashSet<NodeId> = self.graph.adj_list.keys().cloned().collect();

        for &node in &unvisited {
            dist.insert(node, f32::INFINITY);
        }
        dist.insert(self.id, 0.0);

        while let Some(&current) = unvisited.iter().min_by_key(|&&node| dist.get(&node).cloned().unwrap_or(f32::INFINITY) as i32) {
            unvisited.remove(&current);

            if current == destination {
                break;
            }

            for &(neighbor, weight) in &self.graph.get_neighbors(current) {
                let alt = dist.get(&current).cloned().unwrap_or(f32::INFINITY) + weight;
                if alt < dist.get(&neighbor).cloned().unwrap_or(f32::INFINITY) {
                    dist.insert(neighbor, alt);
                    prev.insert(neighbor, current);
                }
            }
        }

        // Ricostruisce il percorso
        let mut path = Vec::new();
        let mut current = destination;
        while let Some(&prev_node) = prev.get(&current) {
            path.push(current);
            current = prev_node;
        }
        path.push(self.id);
        path.reverse();
        path
    }

    // fn send_message(&self, destination: NodeId, message: Vec<u8>) {
    //     let route = self.find_best_route(destination);
    //     let fragments = self.fragment_message(message);
    //
    //     for fragment in fragments {
    //         let packet = Packet {
    //             pack_type: PacketType::MsgFragment(fragment),
    //             routing_header: SourceRoutingHeader {
    //                 hop_index: 0,
    //                 hops: route.clone(),
    //             },
    //             session_id: rand::random(),
    //         };
    //
    //         if let Err(e) = self.packet_send.send(packet) {
    //             println!("Client {}: Error sending packet: {:?}", self.id, e);
    //         }
    //     }
    // }


}
#[derive(Debug, Clone)]
struct Graph {
    adj_list: HashMap<NodeId, Vec<(NodeId, f32)>>, // Lista di adiacenza con PDR come peso
}

impl Graph {
    fn new() -> Self {
        Self {
            adj_list: HashMap::new(),
        }
    }
    fn add_edge(&mut self, from: NodeId, to: NodeId, pdr: f32) {
        self.adj_list
            .entry(from)
            .or_insert_with(Vec::new)
            .push((to, pdr));
    }
    fn get_neighbors(&self, node: NodeId) -> Vec<(NodeId, f32)> {
        self.adj_list.get(&node).cloned().unwrap_or_default()
    }
}