use inline_colorization::*;
extern crate users;
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use super::super::utils::node_config::*;
use super::node::*;
use crate::utils::{common::Channel, queries::MAX_PAGE_SIZE};
use crate::{
    node::messages::{message, node_info::NodeInfo},
    utils::queries::print_query_response,
};

/// This struct represents the Client node in the distributed system.
/// It finds the router and connects to it to send queries.
#[repr(C)]
pub struct Client {
    router_postgres_client: Channel,
    client_info: NodeInfo,
    nodes: NodesConfig,
    ip: String,
    port: String,
}

impl Client {
    /// Creates a new Client node with the given port
    pub fn new(ip: &str, port: &str) -> Self {
        let config = get_nodes_config();
        let stream = Self::get_router_info(get_nodes_config());
        match stream {
            Some(router_stream) => Client {
                router_postgres_client: Channel {
                    stream: Arc::new(Mutex::new(router_stream)),
                },
                client_info: NodeInfo {
                    ip: ip.to_string(),
                    port: port.to_string(),
                    name: format!("{}:{}:client", ip, port),
                },
                nodes: config,
                ip: ip.to_string(),
                port: port.to_string(),
            },
            None => {
                eprintln!("No valid router found in the config. Terminating.");
                panic!("Failed to establish a router connection");
            }
        }
    }

    pub fn get_router_info(config: NodesConfig) -> Option<TcpStream> {
        let mut candidate_ip;
        let mut candidate_port;
        let mut ran_more_than_once = false;

        loop {
            for node in &config.nodes {
                candidate_ip = node.ip.clone();

                let node_port = match node.port.parse::<u64>() {
                    Ok(port) => port,
                    Err(_) => {
                        eprintln!("Invalid port number in the config for {}", node.name);
                        continue;
                    }
                };

                candidate_port = node_port + 1000;

                let mut candidate_stream =
                    match TcpStream::connect(format!("{}:{}", candidate_ip, candidate_port)) {
                        Ok(stream) => stream,
                        Err(_) => {
                            // Connection to node failed
                            continue;
                        }
                    };

                let message = message::Message::new_get_router();
                if candidate_stream
                    .write_all(message.to_string().as_bytes())
                    .is_err()
                {
                    eprintln!("Failed to send message to the node.");
                    continue;
                }

                let response: &mut [u8] = &mut [0; 1024];
                match candidate_stream.set_read_timeout(Some(std::time::Duration::from_secs(3))) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to set read timeout: {:?}", e);
                        continue;
                    }
                };

                match candidate_stream.read(response) {
                    Ok(_) => {
                        let response_str = String::from_utf8_lossy(response);
                        if let Ok(response_message) = message::Message::from_string(&response_str) {
                            match response_message.get_message_type() {
                                message::MessageType::RouterId => {
                                    match Client::handle_router_id_message(response_message) {
                                        Some(router_stream) => {
                                            return Some(router_stream);
                                        }
                                        None => {
                                            continue;
                                        }
                                    }
                                }
                                message::MessageType::NoRouterData => {
                                    // if no router data is found, the network might be in the process of going live.
                                    // wait for a while to allow to elect a leader and try again.
                                    std::thread::sleep(std::time::Duration::from_secs(3));
                                    continue;
                                }
                                _ => {
                                    eprintln!("Invalid response from node.");
                                    continue;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read response from node: {:?}", e);
                    }
                }
            }
            if !ran_more_than_once {
                println!("\n[⚠️] Failed to connect to the database. This will be retried for as long as the program is running.\nYou can stop this program by pressing Ctrl+C.\n");
                ran_more_than_once = true;
            }
        }
    }

    fn handle_router_id_message(response_message: message::Message) -> Option<TcpStream> {
        if let Some(node_info) = response_message.get_data().node_info {
            let node_ip = node_info.ip.clone();
            let node_port = match node_info.port.clone().parse::<u64>() {
                Ok(port) => port,
                Err(_) => {
                    eprintln!("Invalid port number in the config for the router");
                    return None;
                }
            };
            let connections_port = node_port + 1000;

            return match TcpStream::connect(format!("{}:{}", node_ip, connections_port)) {
                Ok(router_stream) => {
                    println!(
                        "{color_bright_green}Connected to router stream {}:{}{style_reset}",
                        node_ip,
                        connections_port.to_string()
                    );
                    return Some(router_stream);
                }
                Err(e) => {
                    eprintln!("Failed to connect to the router stream: {:?}", e);
                    None
                }
            }
        }
        None
    }
    
    fn handle_received_message(buffer: &mut [u8]) {
        let message_string = String::from_utf8_lossy(&buffer);
        let response_message = match message::Message::from_string(&message_string) {
            Ok(message) => message,
            Err(_) => {
                return;
            }
        };

        if response_message.get_message_type() == message::MessageType::QueryResponse {
            let rows = match response_message.get_data().query {
                Some(query) => query,
                None => {
                    return;
                }
            };
            print_query_response(rows);
        }
    }
}

impl NodeRole for Client {
    fn backend(&self) -> Arc<Mutex<postgres::Client>> {
        panic!("Client node does not have a backend");
    }

    fn send_query(&mut self, query: &str) -> Option<String> {
        if query == "whoami;" {
            println!("> I am Client: {}:{}\n", self.ip, self.port);
            return None;
        }

        println!("Sending query from client");
        let message =
            message::Message::new_query(Some(self.client_info.clone()), query.to_string());

        println!(
            "{color_bright_blue}Sending query to router: {}{style_reset}",
            query
        );

        // Intentar obtener el stream actual
        let mut stream = match self.router_postgres_client.stream.lock() {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Failed to lock the stream: {:?}", e);
                return None;
            }
        };

        println!("{color_bright_blue}Stream locked{style_reset}");

        // Intentar enviar el mensaje y reconectar si falla
        if let Err(e) = stream.write_all(message.to_string().as_bytes()) {
            eprintln!(
                "{color_bright_red}Failed to send the query: {:?}{style_reset}",
                e
            );
            eprintln!("Reconnecting to new router...");
            drop(stream);
            // Obtener un nuevo router y actualizar el canal
            if let Some(new_stream) = Self::get_router_info(get_nodes_config()) {
                self.router_postgres_client = Channel {
                    stream: Arc::new(Mutex::new(new_stream)),
                };
                stream = match self.router_postgres_client.stream.lock() {
                    Ok(stream) => stream,
                    Err(e) => {
                        eprintln!("Failed to lock the new stream: {:?}", e);
                        return None;
                    }
                };
                // Reintentar el envío con el nuevo router
                if let Err(e) = stream.write_all(message.to_string().as_bytes()) {
                    eprintln!("Failed to send query after reconnecting: {:?}", e);
                    return None;
                }
                println!("{color_bright_blue}Query sent to new router{style_reset}");
            } else {
                eprintln!("No valid router found during reconnection.");
                return None;
            }
        } else {
            println!("{color_bright_blue}Query sent to router{style_reset}");
        }

        // Preparar el buffer de respuesta
        let mut buffer: [u8; MAX_PAGE_SIZE] = [0; MAX_PAGE_SIZE];

        // Intentar leer la respuesta y reconectar si falla
        match stream.read(&mut buffer) {
            Ok(chars) if chars > 0 => {
                Client::handle_received_message(&mut buffer);
                Some(String::new())
            }
            Ok(_) | Err(_) => {
                eprintln!("{color_bright_red}Failed to read response or empty response received.{style_reset}");
                eprintln!("Reconnecting to new router...");
                drop(stream);
                // Obtener un nuevo router y actualizar el canal
                if let Some(new_stream) = Self::get_router_info(get_nodes_config()) {
                    self.router_postgres_client = Channel {
                        stream: Arc::new(Mutex::new(new_stream)),
                    };
                    println!("{color_bright_green}Reconnected to new router{style_reset}");
                    stream = match self.router_postgres_client.stream.lock() {
                        Ok(stream) => stream,
                        Err(e) => {
                            eprintln!("Failed to lock the new stream: {:?}", e);
                            return None;
                        }
                    };
                    _ = stream.write_all(message.to_string().as_bytes());
                } else {
                    eprintln!("No valid router found during reconnection.");
                }
                None
            }
        }
    }

    
    fn stop(&mut self) {
        // Not implemented
    }
}
