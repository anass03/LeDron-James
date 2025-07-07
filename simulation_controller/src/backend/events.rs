use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::{mpsc, Arc, RwLock};
use image::{DynamicImage, ImageBuffer, RgbImage, Rgba};
use log::{debug, error, info};
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use host::{ContentID, FileId, HostEvent, Image, MediaId, ServerId, ServerToClient};
use host::ServerTypes::{Chat, Content};
use server::Chat as ChatStruct;
use crate::backend::sc_models::{ChatList, FileType, LogType, MediaEntry, NetCacheFile, NetCacheMedia, NodeStatistics, RendererCommand, RendererResponse};
use crate::backend::sc_models::FileType::UTF8;
use crate::backend::sc_models::NetCacheFile::Downloading;
use crate::backend::simulation_controller::SimulationControllerData;

impl SimulationControllerData {
    /// - Drone Events
    pub fn handle_drone_event(&mut self, event: DroneEvent) -> Result<(), (LogType, String)> {
        match event {
            DroneEvent::PacketSent(packet) => {
                match packet.routing_header.current_hop() {
                    Some(drone_id) => {
                        self.packet_sent(drone_id)
                    },
                    None => {}
                }
                self.stats.received_packet();
                Ok(())
            },
            DroneEvent::PacketDropped(packet) => {
                match packet.routing_header.current_hop() {
                    Some(drone_id) => {
                        self.packet_dropped(drone_id)
                    },
                    None => {}
                }
                self.stats.dropped_packet();
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
                info!(target: "main", "[LEDRON-CLI] Servers discovered: {servers:?}");
            },
            HostEvent::ChatUpdate(server_id, string) => {
                let chat: ChatStruct = serde_json::from_str(&string)
                    .map_err(|e| (LogType::ERROR, e.to_string()))?;

                // Wrap in Arc<RwLock<>>
                let chat_arc = Arc::new(RwLock::new(chat));

                match self.tx_to_renderer.as_ref() {
                    Some(sender) => {
                        if let Err(e) = sender.send(RendererCommand::SaveMediaFromChatList(chat_arc.clone())) {
                            return Err((LogType::ERROR, e.to_string()));
                        }
                    },
                    None => {
                        error!(target: "main", "[LEDRON-CLI] Failed to send chat update to renderer");
                    },
                }

                // Extract key for HashMap
                let key = {
                    let chat_ref = chat_arc.read().unwrap();
                    (chat_ref.users.user1, chat_ref.users.user2)
                };

                // Insert into cache
                let mut chat_list = self.chat_cache.local_chats
                    .entry(server_id)
                    .or_insert_with(ChatList::new);

                chat_list.insert_chat_arc(key, chat_arc);
            }
            HostEvent::DownloadingInfo(server_id, content_id, percentage) => {
                self.update_net_cache(server_id, content_id, None, None, Some(percentage))?
            },
            HostEvent::ExpectedPDR(client_id, expected_pdr) => {
                let expected_pdr_clone = expected_pdr.clone();
                self.stats.node_statistic
                    .entry(client_id)
                    .and_modify(|node_stat| node_stat.pdr_expected = expected_pdr)
                    .or_insert_with(|| NodeStatistics {
                        pdr_expected: expected_pdr_clone,
                        ..Default::default()
                    });
            },
            HostEvent::NetSpeed(client_id, upload, download) => {
                self.stats.node_statistic
                    .entry(client_id)
                    .and_modify(|node_stat| {
                        if node_stat.upload_at_sec.len() >= 30 {
                            node_stat.upload_at_sec.remove(0);
                        }
                        node_stat.upload_at_sec.push(upload);
                        if node_stat.download_at_sec.len() >= 30 {
                            node_stat.download_at_sec.remove(0);
                        }
                        node_stat.download_at_sec.push(download);
                    })
                    .or_insert_with(|| NodeStatistics {
                        upload_at_sec: vec![upload],
                        download_at_sec: vec![download],
                        ..Default::default()
                    });
            },
            HostEvent::Route(host_id, route) => {
                self.stats.node_statistic
                    .entry(host_id)
                    .and_modify(|node_stat| node_stat.route = route.clone())
                    .or_insert_with(|| NodeStatistics {
                        route,
                        ..Default::default()
                    });
            },
            _=> {}
        }
        Ok(())
    }

    fn handle_packet_reconstructed(&mut self, server_to_client: ServerToClient) -> Result<(), (LogType, String)> {
        match server_to_client {
            ServerToClient::ServerType(server_type)
            => {
                info!(target: "main", "[LEDRON-CLI] Server type: {:?}", server_type)
            },
            ServerToClient::FilesList(client_id, server_id, file_list)
            => {
                let filtered_file_list: Vec<FileId> = file_list
                    .into_iter()
                    .filter(|file_id| {
                        // Remove hidden files
                        !file_id.starts_with("._") && !file_id.starts_with(".")
                    })
                    .collect();

                self.net_cache
                    .known_file_list
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, filtered_file_list.clone());

                info!(target: "main", "[LEDRON-CLI] Filtered files list from Server {server_id}: {:?}", filtered_file_list);
            },
            ServerToClient::File(server_id, file_id, file)
            => {
                self.update_net_cache(server_id, ContentID::File(file_id), None, Some(file), None)?;

                //self.save_file(file, file_id)?
            },
            ServerToClient::MediaList(client_id, server_id, media_list)
            => {
                // Filter metadata: ._ files
                let filtered_media_list: Vec<MediaId> = media_list
                    .into_iter()
                    .filter(|media_id| {
                        // Remove hidden files
                        !media_id.starts_with("._") &&
                            !media_id.starts_with(".")
                    })
                    .collect();

                self.net_cache
                    .known_media_list
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, filtered_media_list.clone());
                info!(target: "main", "[LEDRON-CLI] Media list from Server {server_id} received: {:?}", filtered_media_list)
            },
            ServerToClient::Media(server_id, media_id, image)
            => {
                self.update_net_cache(server_id, ContentID::Media(media_id), Some(image), None, None)?;

                // Save in local folder
                //self.save_media(&media_id, &image)?;
            },
            ServerToClient::ClientList(client_id, server_id, client_list)
            => {
                let contacts: Vec<NodeId> = client_list.iter().filter(|id| **id != client_id).cloned().collect();
                self.chat_cache
                    .known_contacts
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, contacts);
                info!(target: "main", "[LEDRON-CLI] Client list from CatServer {server_id} received: {:?}", client_list)
            },
            ServerToClient::MessageFrom(server_id, client_id, message)
            => info!(target: "main", "[LEDRON-CLI] Message from Client {client_id} received through Server {server_id}: {message}"),
            ServerToClient::FileFrom(server_id, client_id, ..)
            => {
                info!(target: "main", "[LEDRON-CLI] File from Client {client_id} received through Server {server_id}");
            }
            ServerToClient::MediaFrom(server_id, client_id, ..)
            => {
                info!(target: "main", "[LEDRON-CLI] Media from Client {client_id} received through Server {server_id}");
            }
            ServerToClient::WrongServer(server_id)
            => {
                info!(target: "main", "[LEDRON-CLI] Wrong Server ID: {server_id}");
            }
            ServerToClient::ErrNotFound(server_id)
            => return Err((LogType::ERROR, format!("Err Not Found by Server {server_id}"))),
            ServerToClient::ErrClientId(server_id)
            => return Err((LogType::ERROR, format!("Err Client Id Not Found from Server {server_id}"))),
            _ => {}
        }
        Ok(())
    }

    /// - Channel Handler
    pub fn handle_channels(&mut self) {
        while let Ok(event) = self.host_event_receiver.try_recv() {
            match self.handle_host_event(event) {
                Ok(()) => {},
                Err(e) => error!(target: "main", "[LEDRON-CLI] {e:?}")
            }
        }
        while let Ok(event) = self.drone_event_receiver.try_recv() {
            match self.handle_drone_event(event) {
                Ok(()) => {},
                Err(e) => error!(target: "main", "[LEDRON-CLI] {e:?}")
            }
        }

        self.handle_network_reliability()
    }

    fn update_net_cache(&mut self, _server_id: ServerId, content_id: ContentID, image: Option<Image>, file: Option<Vec<u8>>, percentage: Option<f32>) -> Result<(), (LogType, String)> {
        match content_id {
            ContentID::Media(media_id) => {
                match image {
                    Some(image) => {
                        // Create or update the media entry in downloaded_medias
                        //let media_entry = MediaEntry::new(NetCacheMedia::Downloading(percentage.unwrap_or(0.0)));
                        //self.net_cache.downloaded_medias.insert(media_id.clone(), media_entry);

                        // Send to renderer for processing
                        match self.tx_to_renderer.as_ref() {
                            Some(sender) => {
                                if let Err(e) = sender.send(RendererCommand::EncodeMedia((media_id, image))) {
                                    return Err((LogType::ERROR, e.to_string()));
                                }
                            },
                            None => {
                                return Err((LogType::ERROR, "No tx_to_render sender found".to_string()));
                            },
                        }
                        Ok(())
                    },
                    None => {
                        // Update downloading progress
                        if let Some(mut entry) = self.net_cache.downloaded_medias.get_mut(&media_id) {
                            match entry.cache {
                                NetCacheMedia::Downloading(_) => {},
                                _ => return Ok(())
                            }
                            entry.update_cache_media(NetCacheMedia::Downloading(percentage.unwrap_or(0.0)));
                        } else {
                            // Create new entry if it doesn't exist
                            let media_entry = MediaEntry::new(NetCacheMedia::Downloading(percentage.unwrap_or(0.0)));
                            self.net_cache.downloaded_medias.insert(media_id, media_entry);
                        }
                        Ok(())
                    }
                }
            },
            ContentID::File(file_id) => {
                match file {
                    Some(file) => {
                        // Send to renderer for processing
                        match self.tx_to_renderer.as_ref() {
                            Some(sender) => {
                                if let Err(e) = sender.send(RendererCommand::EncodeFile((file_id, file))) {
                                    return Err((LogType::ERROR, e.to_string()));
                                }
                            },
                            None => {
                                return Err((LogType::ERROR, "No tx_to_render sender found".to_string()));
                            },
                        }
                        Ok(())
                    },
                    None => {
                        // Update downloading progress
                        if let Some(mut entry) = self.net_cache.downloaded_files.get_mut(&file_id) {
                            match *entry {
                                Downloading(_) => {},
                                _ => return Ok(())
                            }
                        }
                        self.net_cache.downloaded_files.insert(file_id, Downloading(percentage.unwrap_or(0.0)));
                        Ok(())
                    }
                }
            }
        }
    }

    fn handle_network_reliability(&mut self) {
        if self.stats.total_packet_exchanged == 0 {
            self.stats.network_reliability = 100;
        } else {
            self.stats.network_reliability = ((1.0
                - (self.stats.total_packet_dropped as f64 / self.stats.total_packet_exchanged as f64)) * 100.0) as u8;
        }
    }

    fn save_file(&self, file: Vec<u8>, file_id: FileId) -> Result<(), (LogType, String)> {
        // Construct filename based on file ID and inferred extension
        let path = self.download_folder.join(&file_id);

        match fs::write(&path, &file) {
            Ok(_) => info!(target: "main", "[LEDRON-CLI] File {file_id} downloaded in : {path:?}"),
            Err(e) => return Err((LogType::INFO, format!("File {file_id} not downloaded: {e}")))
        }
        Ok(())
    }
}

#[inline]
fn get_extension(media_id: &MediaId) -> Option<&str> {
    media_id.rsplitn(2, '.').next().filter(|&ext| ext != media_id)
}

fn open_image(path: &str) {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn().ok();
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", path]).spawn().ok();
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn().ok();
    }
}

