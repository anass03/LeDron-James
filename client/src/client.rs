use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Packet, PacketType, NodeType, FRAGMENT_DSIZE, Fragment};
use std::collections::{HashMap};
use std::fmt::Display;
use std::{fs, io};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use host::{ClientCommand, Message, Host, Functionality, generate_unique_session_id, ServerTypes, Image, SentSessionData};
use host::ServerTypes::Content;
use host::ServerTypes::Chat;
use image::{DynamicImage, ImageBuffer, Rgba};
use serde_json::to_string;

pub struct Client {
    host: Host,
    connected_drone_ids: Vec<NodeId>,
    messages: Vec<Message>,
    servers: HashMap<ServerTypes, NodeId>
}
impl Client {
    pub fn new(id:NodeId,
               type_host:NodeType,
               connected_drone_ids: Vec<NodeId>,
               controller_send:Sender<DroneEvent>,
               controller_recv:Receiver<ClientCommand>,
               packet_recv:Receiver<Packet>,
               packet_send:HashMap<NodeId, Sender<Packet>>) ->Self{
        //Assicura che il client sia conneso a uno o due droni
        assert!(
            connected_drone_ids.len() >= 1 && connected_drone_ids.len() <= 2,
            "Client must be connected to at least one and at most two drones!"
        );

        let host = Host::new(id, type_host, controller_send, controller_recv, packet_recv, packet_send);
        Self{
            host, connected_drone_ids, messages: Vec::new(), servers: HashMap::new()
        }
    }
    pub fn run(&mut self) {
        self.host.initiate_flood();
        loop {
            crossbeam_channel::select_biased! {
                recv(self.host.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.handle_command(command);
                    }
                }
                recv(self.host.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.handle_packet(packet);
                    }
                }
                default => {
                    // Simula inattività, evita spinning continuo
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }
    fn handle_command(&mut self, command: ClientCommand) {
        if self.servers.is_empty(){
            self.request_server_type();
        }
        match command {
            ClientCommand::FilesList => {
                if let Some(&server_id) = self.servers.get(&Content) {
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content: Vec::from("FilesList?".as_bytes()),
                    };
                    if let Err(err) = self.build_msg_fragment(server_id, message) {
                        self.host.log(&format!("Failed to send message: {}", err));
                    }
                } else {
                    self.host.log("No content server found.");
                }
            }
            ClientCommand::File(id) => {
                if let Some(&server_id) = self.servers.get(&Content) {
                    let content = format!("File? id:{}", id);
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content: Vec::from(content.as_bytes()),
                    };
                    if let Err(err) = self.build_msg_fragment(server_id, message) {
                        self.host.log(&format!("Failed to send message: {}", err));
                    }
                } else {
                    self.host.log("No content server found.");
                }
            }
            ClientCommand::MediaList => {
                if let Some(&server_id) = self.servers.get(&Content) {
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content: Vec::from("MediaList?".as_bytes()),
                    };
                    if let Err(err) = self.build_msg_fragment(server_id, message) {
                        self.host.log(&format!("Failed to send message: {}", err));
                    }
                } else {
                    self.host.log("No content server found.");
                }
            }
            ClientCommand::Media(id) => {
                if let Some(&server_id) = self.servers.get(&Content) {
                    let content = format!("Media? id:{}", id);
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content: Vec::from(content.as_bytes()),
                    };
                    if let Err(err) = self.build_msg_fragment(server_id, message) {
                        self.host.log(&format!("Failed to send message: {}", err));
                    }
                } else {
                    self.host.log("No content server found.");
                }
            }
            ClientCommand::RegistrationToChat => {
                if let Some(&server_id) = self.servers.get(&Chat) {
                    let content = format!("RegisterToChat id:{}", self.host.id);
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content: Vec::from(content.as_bytes()),
                    };
                    if let Err(err) = self.build_msg_fragment(server_id, message) {
                        self.host.log(&format!("Failed to send message: {}", err));
                    }
                } else {
                    self.host.log("No chat server found.");
                }
            }
            ClientCommand::ClientList => {
                if let Some(&server_id) = self.servers.get(&Chat) {
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content: Vec::from("ClientList?".as_bytes()),
                    };
                    if let Err(err) = self.build_msg_fragment(server_id, message) {
                        self.host.log(&format!("Failed to send message: {}", err));
                    }
                } else {
                    self.host.log("No chat server found.");
                }
            }
            ClientCommand::MessageFor(client_id, message_content) => {
                if let Some(&server_id) = self.servers.get(&Chat) {
                    let content = format!("MessageFor id:{}, message:{}", client_id, message_content);
                    let message = Message {
                        session_id: generate_unique_session_id(),
                        content: Vec::from(content.as_bytes()),
                    };
                    if let Err(err) = self.build_msg_fragment(server_id, message) {
                        self.host.log(&format!("Failed to send message: {}", err));
                    }
                } else {
                    self.host.log("No chat server found.");
                }
            }
            ClientCommand::ControllerShortcut(packet) => {
                self.handle_packet(packet);
                self.host.initiate_flood();
            }
            _ => {}
        }
    }
    fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type.clone() {
            PacketType::FloodResponse(response) => {
                self.host.process_flood_response(response.clone());
            }
            PacketType::MsgFragment(fragment) => {
                 if let Some(message) = self.host.handle_msg_fragment(packet.clone(),fragment){
                     if let Ok(msg_str) = String::from_utf8(message.clone()) {
                         if msg_str.starts_with("ServerType") {
                             self.register_server_type(msg_str, packet.routing_header.source().unwrap());
                         } else {
                             elaborate(message).expect("Error to elaborate message!");
                         }
                     }
                 }else {
                     self.host.log("Failed to reassemble message fragment."); // ancora non ricevuto tutti i frammenti di quella sessione
                 }
            }
            PacketType::FloodRequest(flood_request) => {
                self.host.process_flood_request(flood_request, packet);
            }
            PacketType::Ack(ack) => {
                self.host.handle_ack(ack, packet.session_id);
            }
            PacketType::Nack(nack) => {
                self.host.handle_nack(nack, packet.session_id);
            }
        }
    }
    fn request_server_type(&mut self) {
        for server_id in self.host.network.servers.clone() {
            let message = Message {
                session_id: generate_unique_session_id(),
                content: Vec::from("ServerType".as_bytes()), // Richiesta del tipo di server
            };

            if let Err(err) = self.build_msg_fragment(server_id, message) {
                self.host.log(&format!("Failed to request server type from {}: {}", server_id, err));
            }
        }
    }
    fn register_server_type(&mut self, response: String, server_id: NodeId) {
        if response.contains("Content") {
            self.servers.insert(Content, server_id);
            self.host.log(&format!("Registered Content server at {}", server_id));
        } else if response.contains("Chat") {
            self.servers.insert(Chat, server_id);
            self.host.log(&format!("Registered Chat server at {}", server_id));
        } else {
            self.host.log(&format!("Unknown server type from {}", server_id));
        }
    }
    fn build_msg_fragment(&mut self, destination: NodeId, message: Message) -> Result<(), String> {
        // Trova il miglior percorso per la destinazione
        let route = self.host.network.find_best_route(self.host.id, destination);
        if route.is_empty() {
            return Err("No route found to destination".into());
        }

        // Serializza il contenuto del messaggio
        let content = to_string(&message.content).unwrap();
        let content_bytes = content.as_bytes();
        let total_n_fragments = (content_bytes.len() + FRAGMENT_DSIZE - 1) / FRAGMENT_DSIZE;

        // Aggiungi o aggiorna l'entry nella mappa dei frammenti inviati
        let mut sent_fragments = self.host.sent_fragments.lock().unwrap();
        let session_data = sent_fragments.entry(message.session_id).or_insert_with(|| SentSessionData {
            buffer: vec![None; total_n_fragments],
            last_activity: Instant::now(),
            route: route.clone(), // Salva il percorso pre-calcolato
            destination,          // Salva il destinatario
        });

        // Aggiorna `last_activity` e assicura che il buffer sia grande abbastanza
        session_data.last_activity = Instant::now();
        if session_data.buffer.len() < total_n_fragments {
            session_data.buffer.resize(total_n_fragments, None);
        }

        // Invia i frammenti
        for (i, chunk) in content_bytes.chunks(FRAGMENT_DSIZE).enumerate() {
            let fragment = Fragment {
                fragment_index: i as u64,
                total_n_fragments: total_n_fragments as u64,
                length: chunk.len() as u8,
                data: {
                    let mut data = [0; FRAGMENT_DSIZE];
                    data[..chunk.len()].copy_from_slice(chunk);
                    data
                },
            };

            // Aggiungi il frammento al buffer
            session_data.buffer[i] = Some(fragment.clone());

            // Costruisci e invia il pacchetto
            let routing_header = SourceRoutingHeader::new(route.clone(), 0);
            let packet = Packet::new_fragment(routing_header, message.session_id, fragment);
            self.host.send_packet(packet, None);
        }

        Ok(())
    }
}

fn elaborate(message: Vec<u8>) -> Result<String, String> {
    match message.first().copied() {
        Some(b'c') => rev_message(message).map_err(|e| e.to_string()),
        Some(b'm') => {
            let image = revert_image(message)?; // Decodifica l'immagine
            image.save("output.png").map_err(|e| e.to_string())?; // Salva su file
            Ok("Image processed and saved as output.png".into())
        },
        Some(b'f') => revert_file(message).map(|_| "File saved".into())
            .map_err(|e| format!("File error: {}", e)),
        _ => Err("Invalid message type".into()),
    }
}

// Aggiunto tipo di ritorno esplicito e gestione errori
fn rev_message(bytes: Vec<u8>) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(bytes)
}
fn revert_file(bytes: Vec<u8>) -> io::Result<()> {
    // Write the bytes directly to a file
    fs::write("output.txt", bytes)?; //Il file viene salvato con il nome "output.txt" nella directory di esecuzione del programma.
    Ok(())
}

fn revert_image(data: Vec<u8>) -> Result<DynamicImage, String> {
    if data.len() < 9 {
        return Err("Invalid image header".into());
    }

    let width = u32::from_be_bytes(data[1..5].try_into().unwrap());
    let height = u32::from_be_bytes(data[5..9].try_into().unwrap());
    let image_data = &data[9..];

    if image_data.len() != (width * height * 4) as usize {
        return Err(format!(
            "Dati immagine non validi: attesi {}, ottenuti {}",
            width * height * 4,
            image_data.len()
        ));
    }

    ImageBuffer::from_raw(width, height, image_data.to_vec())
        .map(DynamicImage::ImageRgba8)
        .ok_or_else(|| "Failed to create image buffer".into())
}
// fn revert_image(data:Vec<u8>) -> Result<DynamicImage, String> {
//     let image= Image{
//         width: data[1] as u32,
//         height: data[2] as u32,
//         data: data[3..].to_vec(),
//     };
//     if image.data.len() != (image.width * image.height * 4) as usize {
//         return Err(format!(
//             "Byte vector size mismatch: expected {}, got {}",
//             image.width * image.height * 4,
//             image.data.len()
//         ));
//     }
//     // Try creating the image buffer
//     let img_buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(image.width, image.height, image.data)
//         .ok_or("Failed to create ImageBuffer from raw bytes")?;
//
//     Ok(DynamicImage::ImageRgba8(img_buffer))
// }