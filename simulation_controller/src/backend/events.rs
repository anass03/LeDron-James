use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use image::{DynamicImage, ImageBuffer, RgbImage, Rgba};
use log::{debug, error, info};
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use host::{ContentID, FileId, HostEvent, Image, MediaId, ServerId, ServerToClient};
use host::ServerTypes::{Chat, Content};
use server::Chat as ChatStruct;
use crate::backend::sc_models::{ChatList, FileType, LogType, NetCacheFile, NetCacheMedia, NodeStatistics, RendererCommand, RendererResponse};
use crate::backend::sc_models::FileType::UTF8;
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
                self.update_net_cache(server_id, content_id, None, None, Some(percentage))?
            },
            HostEvent::UploadingInfo(server_id, client_id, percentage) => {},
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
                    .and_modify(|node_stat| node_stat.routh = route.clone())
                    .or_insert_with(|| NodeStatistics {
                        routh: route,
                        ..Default::default()
                    });
            },
        }
        Ok(())
    }

    fn handle_packet_reconstructed(&mut self, server_to_client: ServerToClient) -> Result<(), (LogType, String)> {
        match server_to_client {
            ServerToClient::ServerType(server_type)
            => {
                info!(target: "main", "Server type: {:?}", server_type)
            },
            ServerToClient::FilesList(client_id, server_id, file_list)
            => {
                let filtered_file_list: Vec<FileId> = file_list
                    .into_iter()
                    .filter(|file_id| {
                        // Rimuovi file macOS e file nascosti
                        !file_id.starts_with("._") && !file_id.starts_with(".")
                    })
                    .collect();

                self.net_cache
                    .known_file_list
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, filtered_file_list.clone());

                info!(target: "main", "Filtered files list from Server {server_id}: {:?}", filtered_file_list);
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
                        // Rimuovi file che iniziano con ._
                        !media_id.starts_with("._") &&
                            // Opzionalmente, rimuovi anche altri file nascosti
                            !media_id.starts_with(".")
                    })
                    .collect();

                self.net_cache
                    .known_media_list
                    .entry(client_id)
                    .or_default()
                    .insert(server_id, filtered_media_list.clone());
                info!(target: "main", "Media list from Server {server_id} received: {:?}", filtered_media_list)
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
                info!(target: "main", "Client list from CatServer {server_id} received: {:?}", client_list)
            },
            ServerToClient::MessageFrom(server_id, client_id, message)
            => info!(target: "main", "Message from Client {client_id} received through Server {server_id}: {message}"),
            ServerToClient::FileFrom(server_id, client_id, ..)
            => {
                self.logs.push_sc_log(LogType::INFO, format!("File from Client {client_id} received through Server {server_id}"));
            }
            ServerToClient::MediaFrom(server_id, client_id, ..)
            => {
                self.logs.push_sc_log(LogType::INFO, format!("Media from Client {client_id} received through Server {server_id}"));
            }
            ServerToClient::WrongServer(server_id)
            => {
                self.logs.push_sc_log(LogType::INFO, format!("Wrong Server ID: {server_id}"));
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
                Err(e) => {self.logs.push_sc_log(e.0, e.1);},
            }
        }
        while let Ok(event) = self.drone_event_receiver.try_recv() {
            match self.handle_drone_event(event) {
                Ok(()) => {},
                Err(e) => {self.logs.push_sc_log(e.0, e.1)}
            }
        }

        self.handle_network_reliability()
    }

    fn update_net_cache(&mut self, server_id: ServerId, content_id: ContentID, image: Option<Image>, file: Option<Vec<u8>>, percentage: Option<f32>) -> Result<(), (LogType, String)> {
        match content_id {
            ContentID::Media(media_id) => {
                let server_map = match self.net_cache.downloaded_medias.get_mut(&server_id) {
                    Some(map) => map,
                    None => {
                        return Err((LogType::ERROR, format!("Server ID {server_id} not found in downloaded_medias")));
                    }
                };

                match server_map.get_mut(&media_id) {
                    Some((channels, net_cache_media)) => {
                        let (resize_request_sender, resize_request_receiver) = mpsc::channel();
                        let (render_update_sender, render_update_receiver) = mpsc::channel();
                        let media_channels = (Some(resize_request_sender.clone()), Some(render_update_receiver));
                        let renderer_channels = (render_update_sender, resize_request_receiver);
                            match self.tx_to_renderer.as_ref() {
                                Some(mut sender) => {
                                    if let (None, None) = channels {
                                        if let Err(e) = sender.send(RendererCommand::AddChannel(renderer_channels)) {
                                            return Err((LogType::ERROR, e.to_string()));
                                        }
                                        *channels = media_channels;
                                    }
                                    match image {
                                        Some(image) => {
                                            if let Err(e) = sender.send(RendererCommand::EncodeMedia((resize_request_sender, server_id, media_id, image))) {
                                                return Err((LogType::ERROR, e.to_string()));
                                            }
                                        },
                                        None => {
                                            *net_cache_media = NetCacheMedia::Downloading(percentage.unwrap_or(0.0))
                                        }
                                    }
                                },
                                None => {
                                    self.logs.push_sc_log(LogType::ERROR, "No tx_to_render sender found".to_string());
                                },
                            }
                        Ok(())
                    }
                    None => {
                        Err((LogType::ERROR, format!("Media ID {media_id} entry for NetCacheMedia not found")))
                    }
                }
            },
            ContentID::File(file_id) => {
                let server_map = match self.net_cache.downloaded_files.get_mut(&server_id) {
                    Some(map) => map,
                    None => {
                        return Err((LogType::ERROR, format!("Server ID {server_id} not found in downloaded_files")));
                    }
                };

                match server_map.get_mut(&file_id) {
                    Some(net_cache_file) => {
                        match file {
                            Some(file) => *net_cache_file = NetCacheFile::Downloaded(file),
                            None => {
                                *net_cache_file = NetCacheFile::Downloading(percentage.unwrap_or(0.0))
                            }
                        }
                        Ok(())
                    }
                    None => {
                        Err((LogType::ERROR, format!("File ID {file_id} entry for NetCacheFile not found")))
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

    fn save_media(&self, media_id: &MediaId, image: &Image) -> Result<(), (LogType, String)> {
        // Get file extension
        let extension = get_extension(media_id)
            .ok_or_else(|| return (LogType::INFO, format!("Unknown extension for file {media_id}")))?;

        // Construct path safely
        let path = self.download_folder.join(media_id);

        // Ensure download directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| (LogType::ERROR, format!("Failed to create directory {}: {}", parent.display(), e)))?;
        }

        // Create image buffer without cloning if possible
        let rgba_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(image.width, image.height, image.data.clone())
            .ok_or_else(|| (LogType::ERROR, format!("Failed to create image buffer from media {}", media_id)))?;

        // Convert to DynamicImage
        let dynamic_image = DynamicImage::ImageRgba8(rgba_buffer);

        // Save based on extension
        let save_result = match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" => {
                // Convert to RGB to drop alpha channel for JPEG
                let rgb_image: RgbImage = dynamic_image.to_rgb8();
                rgb_image.save(&path)
            },
            _ => {
                // For PNG and other formats that support alpha
                dynamic_image.save(&path)
            }
        };

        save_result
            .map_err(|e| (LogType::ERROR, format!("Error saving media {}: {}", media_id, e)))?;

        info!(target: "main", "Image saved: {}", path.display());
        Ok(())
    }

    fn save_file(&self, file: Vec<u8>, file_id: FileId) -> Result<(), (LogType, String)> {
        // Construct filename based on file ID and inferred extension
        let path = self.download_folder.join(&file_id);

        match fs::write(&path, &file) {
            Ok(_) => info!(target: "main", "File {file_id} downloaded in : {path:?}"),
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

fn open_file(path: &str) {
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
}

/*fn file_type(file: Vec<u8>) -> FileType {
    // Check if valid UTF-8
    match str::from_utf8(&file) {
        Ok(utf8_str) => {
            // Split in lines Vec<String>
            let lines: Vec<String> = utf8_str
                .lines()
                .map(|line| line.to_string())
                .collect();

            UTF8(lines)
        }
        Err(e) => {
            std::fs::File::
        }
    }
}*/