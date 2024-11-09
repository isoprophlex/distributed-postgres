use inline_colorization::*;
extern crate users;
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use super::super::utils::node_config::*;
use super::node::*;
use crate::utils::common::Channel;
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
            Some(router_stream) => {
                Client {
                    router_postgres_client: Channel {
                        stream: Arc::new(Mutex::new(router_stream)),
                    },
                    client_info: NodeInfo {
                        ip: ip.to_string(),
                        port: port.to_string(),
                    },
                    nodes: config,
                    ip: ip.to_string(),
                    port: port.to_string(),
                }
            }
            None => {
                eprintln!("No valid router found in the config. Terminating.");
                panic!("Failed to establish a router connection");
            }
        }



    }
    pub fn get_router_info(config: NodesConfig) -> Option<TcpStream> {
        let mut candidate_ip;
        let mut candidate_port;
        for node in config.nodes {
            candidate_ip = node.ip.clone();
            candidate_port = node.port.clone().parse::<u64>().unwrap() + 1000;
            let mut candidate_stream =
                match TcpStream::connect(format!("{}:{}", candidate_ip, candidate_port)) {
                    Ok(stream) => {
                        println!(
                            "{color_bright_green}Health connection established with {}:{}{style_reset}",
                            candidate_ip, candidate_port
                        );
                        stream
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to the router: {:?}", e);
                        continue;
                    }
                };
            let message = message::Message::new_get_router();
            candidate_stream
                .write_all(message.to_string().as_bytes())
                .unwrap();
            let response: &mut [u8] = &mut [0; 1024];
            candidate_stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            match candidate_stream.read(response) {
                Ok(_) => {
                    let response_str = String::from_utf8_lossy(response);
                    let response_message = message::Message::from_string(&response_str).unwrap();

                    if response_message.get_message_type() == message::MessageType::RouterId {
                        let node_info: NodeInfo = response_message.get_data().node_info.unwrap();
                        let node_ip = node_info.ip.clone();
                        let node_port = node_info.port.clone();
                        let connections_port = node_port.parse::<u64>().unwrap() + 1000;
                        let router_stream =
                            match TcpStream::connect(format!("{}:{}", node_ip, connections_port)) {
                                Ok(stream) => {
                                    println!(
                                        "{color_bright_green}Router stream {}:{}{style_reset}",
                                        node_ip,
                                        connections_port.to_string()
                                    );
                                    stream
                                }
                                Err(e) => {
                                    eprintln!("Failed to connect to the router: {:?}", e);
                                    panic!("Failed to connect to the router");
                                }
                            };
                        return Some(router_stream)
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn handle_received_message(buffer: &mut [u8]) {
        let message_string = String::from_utf8_lossy(&buffer);
        let response_message = match message::Message::from_string(&message_string) {
            Ok(message) => message,
            Err(e) => {
                return;
            }
        };

        if response_message.get_message_type() == message::MessageType::QueryResponse {
            let rows = response_message.get_data().query.unwrap();
            print_query_response(rows);
        }
    }
}

impl NodeRole for Client {
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
            eprintln!("Failed to send the query: {:?}", e);
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
        let mut buffer: [u8; 1024] = [0; 1024];

        // Intentar leer la respuesta y reconectar si falla
        match stream.read(&mut buffer) {
            Ok(chars) if chars > 0 => {
                Client::handle_received_message(&mut buffer);
                Some(String::new())
            }
            Ok(_) | Err(_) => {
                eprintln!("Failed to read response or empty response received.");
                eprintln!("Reconnecting to new router...");
                drop(stream);
                // Obtener un nuevo router y actualizar el canal
                if let Some(new_stream) = Self::get_router_info(get_nodes_config()) {
                    self.router_postgres_client = Channel {
                        stream: Arc::new(Mutex::new(new_stream)),
                    };
                    println!("{color_bright_green}Reconnected to new router{style_reset}");
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


