use std::collections::HashMap;
use std::sync::mpsc;
use log::{error, info};
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use host::{ContentID, HostEvent, MediaId, ServerToClient};
use host::ServerTypes::{Chat, Content};
use server::Chat as ChatStruct;
use crate::backend::sc_models::{ChatList, LogType, NetCacheMedia, RendererCommand, RendererResponse};
use crate::backend::simulation_controller::SimulationControllerData;

impl SimulationControllerData {
    /// - Drone Events
    pub fn handle_drone_event(&mut self, event: DroneEvent) -> Result<(), (LogType, String)> {
        match event {
            DroneEvent::PacketSent(_packet) => {
                self.stats.received_packet();
                Ok(())
            },
            DroneEvent::PacketDropped(_packet) => {
                Ok(())
            },
            DroneEvent::ControllerShortcut(packet) => {
                let receiver_id = match packet.routing_header.hops.last() {
                    Some(receiver_id) => receiver_id,
                    None => return Err((LogType::ERROR, "No routing header found for controller shortcut".to_string())),
                };
                match self.controller_shortcut(*receiver_id, packet) {
                    Ok(()) => Ok(()),
                    Err(e) => Err((LogType::ERROR, e)),
                }

            },
        }
    }

    /// - Host Events
    pub fn handle_host_event(&mut self, event: HostEvent) -> Result<(), (LogType, String)> {
        match event {
            HostEvent::PacketReconstructed(server_to_client) => {
                self.handle_packet_reconstructed(server_to_client)?
            },
            HostEvent::PacketSent(_packet) => {
                self.stats.received_packet();
            },
            HostEvent::ServersDiscovered(client_id, servers) => {
                if let Some(chat_servers) = servers.get(&Chat) {
                    self.chat_cache
                        .known_chat_servers
                        .insert(client_id, chat_servers.clone());
                }
                if let Some(content_servers) = servers.get(&Content) {
                    self.net_cache
                        .known_content_servers
                        .insert(client_id, content_servers.clone());
                }
                info!(target: "main", "Servers discovered: {servers:?}");
            },
            HostEvent::ChatUpdate(server_id, string) => {
                let chat: ChatStruct = serde_json::from_str(&string)
                    .map_err(|e| (LogType::ERROR, e.to_string()))?;

                let key = (chat.users.user1, chat.users.user2);
                let mut chats_stored = HashMap::new();
                chats_stored.insert(key, chat);

                let chat_list = ChatList { chats_stored };
                self.chat_cache.local_chats.insert(server_id, chat_list);
            }
            HostEvent::DownloadingInfo(server_id, content_id, percentage) => {
                //debug!(target: "main", "[LeDron-CLI] - 💥Downloading: {:?}", percentage);
                let media_id: MediaId = match content_id {
                    ContentID::Media(id) => id,
                    ContentID::File(_) => return Err((LogType::WARN, "The content downloading is a File, not a Media".to_string()))
                };

                let server_map = match self.net_cache.downloaded_medias.get_mut(&server_id) {
                    Some(map) => map,
                    None => {
                        return Err((LogType::ERROR, format!("Server ID {server_id} not found in downloaded_medias")));
                    }
                };

                match server_map.get_mut(&media_id) {
                    Some((channels, net_cache_media)) => {
                        if let NetCacheMedia::Downloading(_) = net_cache_media {
                            if let (None, None) = channels {
                                let (resize_request_sender, resize_request_receiver) = mpsc::channel();
                                let (render_update_sender, render_update_receiver) = mpsc::channel();
                                let media_channels = (Some(resize_request_sender), Some(render_update_receiver));
                                let renderer_channels = (render_update_sender, resize_request_receiver);

                                match self.tx_to_renderer.as_ref() {
                                    Some(sender) => {
                                        if let Err(e) = sender.send(RendererCommand::AddChannel(renderer_channels)) {
                                            return Err((LogType::ERROR, e.to_string()));
                                        }
                                    },
                                    None => {
                                        self.logs.push_sc_log(LogType::ERROR, "No tx_to_render sender found".to_string());
                                    }
                                }
                                *channels = media_channels;
                            }
                            *net_cache_media = NetCacheMedia::Downloading(percentage);
                        }
                    }
                    None => {
                        return Err((LogType::ERROR, format!("Media ID {media_id} entry for NetCacheMedia not found")));
                    }
                }
            },
            HostEvent::UploadingInfo(server_id, client_id, percentage) => {},
            HostEvent::ExpectedPDR(expected_pdr) => {},
            HostEvent::NetSpeed(upload, download) => {},
            HostEvent::Route(host_id, route) => {},
        }
        Ok(())
    }

    fn handle_packet_reconstructed(&mut self, server_to_client: ServerToClient) -> Result<(), (LogType, String)> {
        match server_to_client {
            ServerToClient::ServerType(server_type)
            => {
                info!(target: "main", "Server type: {:?}", server_type) },
            ServerToClient::FilesList(client_id, server_id, file_list)
            => {
                self.net_cache
                    .known_file_list
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, file_list.clone());
                info!(target: "main", "Files list from Server {server_id} received: {file_list:?}") },
            ServerToClient::File(server_id, file_id, file)
            => {

                /*let (resize_request_sender, resize_request_receiver) = mpsc::channel();
                let (render_update_sender, render_update_receiver) = mpsc::channel();
                let media_channels = (Some(resize_request_sender), Some(render_update_receiver));
                let renderer_channels = (render_update_sender, resize_request_receiver);
                let net_cache_media = NetCacheMedia::Downloaded(file);
                let mut media_map = HashMap::new();
                media_map.insert(file_id, (media_channels, net_cache_media));
                match self.tx_to_renderer.as_ref() {
                    Some(sender) => {
                        match sender.send(RendererCommand::AddChannel(renderer_channels)) {
                            Ok(_) => {},
                            Err(e) => {return Err((LogType::ERROR, e.to_string()))}
                        }
                    },
                    None => {
                        self.logs.push_sc_log(LogType::ERROR, "No tx_to_render sender found".to_string());
                    }
                }
                self.net_cache.downloaded_medias.insert(server_id, media_map);*/

                // Construct filename based on file ID and inferred extension
                /*let path = PathBuf::from("assets").join(&file_id);

                fs::write(&path, &file).expect("file not downloaded");

                // Open with system default application
                #[cfg(target_os = "windows")]
                {
                    let path_str = path.to_str().unwrap();
                    //let quoted_path = format!("\"{}\"", path_str);
                    Command::new("cmd")
                        .args(&["/C", "start", "", &path_str])
                        .spawn()
                        .expect("Unable to open file on Windows");
                }

                #[cfg(target_os = "macos")]
                {
                    Command::new("open")
                        .arg(&path)
                        .spawn()
                        .expect("Unable to open file on macOS");
                }

                #[cfg(target_os = "linux")]
                {
                    Command::new("xdg-open")
                        .arg(&path)
                        .spawn()
                        .expect("Unable to open file on Linux");
                }

                self.logs.push_sc_log(LogType::INFO, format!("File {file_id} received from Server {server_id}"));
*/
            },
            ServerToClient::MediaList(client_id, server_id, media_list)
            => {
                self.net_cache
                    .known_media_list
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, media_list.clone());
                info!(target: "main", "Media list from Server {server_id} received: {:?}", media_list) },
            ServerToClient::Media(server_id, media_id, image)
            => {
                let server_map = match self.net_cache.downloaded_medias.get_mut(&server_id) {
                    Some(map) => map,
                    None => {
                        return Err((LogType::ERROR, format!("Server ID {server_id} not found in downloaded_medias")));
                    }
                };

                match server_map.get_mut(&media_id) {
                    Some((channels, net_cache_media)) => {
                        if let (None, None) = channels {
                            let (resize_request_sender, resize_request_receiver) = mpsc::channel();
                            let (render_update_sender, render_update_receiver) = mpsc::channel();
                            let media_channels = (Some(resize_request_sender), Some(render_update_receiver));
                            let renderer_channels = (render_update_sender, resize_request_receiver);

                            match self.tx_to_renderer.as_ref() {
                                Some(sender) => {
                                    if let Err(e) = sender.send(RendererCommand::AddChannel(renderer_channels)) {
                                        return Err((LogType::ERROR, e.to_string()));
                                    }
                                },
                                None => {
                                    self.logs.push_sc_log(LogType::ERROR, "No tx_to_render sender found".to_string());
                                }
                            }
                            *channels = media_channels;
                        }
                        *net_cache_media = NetCacheMedia::Downloaded(image);
                    }
                    None => {
                        return Err((LogType::ERROR, format!("Media ID {media_id} entry for NetCacheMedia not found")));
                    }
                }

                //----------------------------------------------------------------------
                /*let extension = match get_extension(&media_id) {
                    Some(extension) => extension,
                    None => return Err((LogType::INFO, format!("Unknown extension for file {media_id}"))),
                };
                let path = format!("assets/{}", media_id);
                let path_ref = Path::new(&path);

                if let Some(rgba_buffer) = ImageBuffer::<Rgba<u8>, _>::from_raw(image.width, image.height, image.data.clone()) {
                    // Convert to DynamicImage
                    let dynamic_image = DynamicImage::ImageRgba8(rgba_buffer);

                    // If extension is jpg/jpeg, convert to RGB to drop alpha
                    let save_result = match extension.to_lowercase().as_str() {
                        "jpg" | "jpeg" => {
                            let rgb_image: RgbImage = dynamic_image.to_rgb8();
                            rgb_image.save(path_ref)
                        },
                        _ => dynamic_image.save(path_ref), // png, etc.
                    };

                    match save_result {
                        Ok(_) => {
                            self.logs.push_sc_log(LogType::INFO, format!("Media {} from Server {server_id} saved as {}", media_id, path));
                            open_image(&path)
                        },
                        Err(e) => self.logs.push_sc_log(LogType::ERROR, format!("Error saving media {}: {}", media_id, e)),
                    }
                } else {
                    self.logs.push_sc_log(LogType::ERROR, format!("Impossible to  generate image from media {}", media_id));
                }*/
            },
            ServerToClient::ClientList(client_id, server_id, client_list)
            => {
                let contacts: Vec<NodeId> = client_list.iter().filter(|id| **id != client_id).cloned().collect();
                self.chat_cache
                    .known_contacts
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, contacts);
                info!(target: "main", "Client list from CatServer {server_id} received: {:?}", client_list) },
            ServerToClient::MessageFrom(server_id, client_id, message)
            => info!(target: "main", "Message from Client {client_id} received through Server {server_id}: {message}"),
            ServerToClient::FileFrom(server_id, client_id,..)
            => {
                self.logs.push_sc_log(LogType::INFO, format!("File from Client {client_id} received through Server {server_id}"));
            }
            ServerToClient::MediaFrom(server_id, client_id, ..)
            => {
                self.logs.push_sc_log(LogType::INFO, format!("Media from Client {client_id} received through Server {server_id}"));
            }
            ServerToClient::Sending(content_id, d) => {}
            ServerToClient::WrongServer(server_id)
            => {
                self.logs.push_sc_log(LogType::INFO, format!("Wrong Server ID: {server_id}"));
            }
            ServerToClient::ErrNotFound(server_id)
            => return Err((LogType::ERROR, format!("Err Not Found by Server {server_id}"))),
            ServerToClient::ErrClientId(server_id)
            => return Err((LogType::ERROR, format!("Err Client Id Not Found from Server {server_id}"))),
        }
        Ok(())
    }

    /// - Media Renderer Handler
    pub fn handle_rx_renderer(&mut self){
        match &self.rx_from_renderer{
            None => {}
            Some(channel) => {
                while let Ok(event) = channel.try_recv() {
                    match event{
                        RendererResponse::Encoded((enc_server, enc_media, enc_proto)) => {
                            error!(target: "main", "Received encoded Response");
                            match self.net_cache.downloaded_medias.get_mut(&enc_server) {
                                None => {

                                }
                                Some(hash) => {
                                    match hash.remove(&enc_media){
                                        None => {}
                                        Some(((ch1,ch2),net)) => {
                                            hash.insert(enc_media, ((ch1,ch2), NetCacheMedia::Rendered(enc_proto)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// - Channel Handler
    pub fn handle_channels(&mut self) {
        while let Ok(event) = self.host_event_receiver.try_recv() {
            match self.handle_host_event(event) {
                Ok(()) => {},
                Err(e) => {self.logs.push_sc_log(e.0, e.1);},
            }
        }
        while let Ok(event) = self.drone_event_receiver.try_recv() {
            match self.handle_drone_event(event) {
                Ok(()) => {},
                Err(e) => {self.logs.push_sc_log(e.0, e.1)}
            }
        }
    }
}