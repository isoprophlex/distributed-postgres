use super::memory_manager::MemoryManager;
use super::messages::message::{Message, MessageType};
use super::messages::node_info::NodeInfo;
use super::node::NodeRole;
use super::tables_id_info::TablesIdInfo;
use crate::node::messages::node_info::find_name_for_node;
use crate::utils::common::{connect_to_node, ConvertToString};
use crate::utils::node_config::{get_memory_config, get_nodes_config};
use indexmap::IndexMap;
use inline_colorization::*;
use postgres::Client as PostgresClient;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{io, thread};

extern crate users;

/// This struct represents the Shard node in the distributed system. It will communicate with the router
#[repr(C)]
#[derive(Clone)]
pub struct Shard {
    backend: Arc<Mutex<PostgresClient>>,
    ip: Arc<str>,
    port: Arc<str>,
    name: String,
    memory_manager: Arc<Mutex<MemoryManager>>,
    router_info: Arc<Mutex<Option<NodeInfo>>>,
    tables_max_id: Arc<Mutex<TablesIdInfo>>,
    pub stopped: Arc<Mutex<bool>>,
}

use std::fmt;
impl fmt::Debug for Shard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Shard")
            .field("ip", &self.ip)
            .field("port", &self.port)
            .field("router_info", &self.router_info)
            .field("tables_max_id", &self.tables_max_id)
            .finish()
    }
}

impl Shard {
    /// Creates a new Shard node in the given port.
    #[must_use]
    pub fn new(ip: &str, port: &str) -> Self {
        println!("Creating a new Shard node in port: {port}");
        println!("Connecting to the database in port: {port}");

        let backend: PostgresClient = match connect_to_node(ip, port) {
            Ok(backend) => backend,
            Err(e) => {
                eprintln!("Failed to connect to the database: {e}");
                panic!("Failed to connect to the database");
            }
        };

        let memory_manager = Self::initialize_memory_manager();

        println!(
            "{color_blue}[Shard] Available Memory: {:?} %{style_reset}",
            memory_manager.available_memory_perc
        );

        let name = match find_name_for_node(ip.to_string(), port.to_string()) {
            Some(name) => name,
            None => {
                eprintln!("Failed to find name for node. Using ip and port for identification");
                format!("{}:{}", ip, port)
            }
        };

        let mut shard = Shard {
            backend: Arc::new(Mutex::new(backend)),
            ip: Arc::from(ip),
            port: Arc::from(port),
            name,
            memory_manager: Arc::new(Mutex::new(memory_manager)),
            router_info: Arc::new(Mutex::new(None)),
            tables_max_id: Arc::new(Mutex::new(IndexMap::new())),
            stopped: Arc::new(Mutex::new(false)),
        };

        let _ = shard.update();

        println!("{color_bright_green}Shard created successfully. Shard: {shard:?}{style_reset}");

        shard
    }

    fn initialize_memory_manager() -> MemoryManager {
        let config = get_memory_config();
        let reserved_memory = config.unavailable_memory_perc;
        MemoryManager::new(reserved_memory)
    }

    pub fn look_for_sharding_network(ip: &str, port: &str, name: &str) {
        println!("Checking if there's a sharding network ...");

        let config = get_nodes_config();
        let mut candidate_ip;
        let mut candidate_port;

        for node in config.nodes {
            candidate_ip = node.ip.clone();
            let node_port = match node.port.parse::<u64>() {
                Ok(port) => port,
                Err(_) => {
                    eprintln!("Failed to parse port number for node: {}", node.ip);
                    continue;
                }
            };

            // Ignore self
            if (&candidate_ip == ip) && (&node_port.to_string() == port) {
                continue;
            }

            candidate_port = node_port + 1000;

            let mut candidate_stream =
                match TcpStream::connect(format!("{}:{}", candidate_ip, candidate_port)) {
                    Ok(stream) => {
                        println!(
                        "{color_bright_green}Health connection established with {}:{}{style_reset}",
                        candidate_ip, candidate_port
                    );
                        stream
                    }
                    Err(_) => {
                        continue;
                    }
                };

            let hello_message = Message::new_hello_from_node(NodeInfo {
                ip: ip.to_string(),
                port: port.to_string(),
                name: name.to_string(),
            });
            println!("{color_bright_green}Sending HelloFromNode message to {candidate_ip}:{candidate_port}{style_reset}");

            match candidate_stream.write_all(hello_message.to_string().as_bytes()) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!(
                        "Failed to send HelloFromNode message to node {}: {e}",
                        candidate_ip
                    );
                }
            };
        }
    }

    pub fn accept_connections(shared_shard: Arc<Mutex<Shard>>, ip: String, accepting_port: String) {
        let port = match accepting_port.parse::<u64>() {
            Ok(port) => port + 1000,
            Err(_) => {
                eprintln!("Failed to parse port number: {}", accepting_port);
                return;
            }
        };

        let listener = match TcpListener::bind(format!("{}:{}", ip, port)) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind listener: {e}");
                return;
            }
        };

        let shard = match shared_shard.lock() {
            Ok(shared_router) => shared_router,
            Err(_) => {
                eprintln!("Failed to get shared shard");
                drop(listener);
                return;
            }
        };

        let name = shard.name.clone();
        let stopped = shard.stopped.clone();
        drop(shard);

        // After binding a listener, look for an ongoing sharding network live
        Shard::look_for_sharding_network(&ip, &accepting_port, &name);

        let mut handles: Vec<JoinHandle<()>> = Vec::new();

        loop {
            let must_stop = match stopped.lock() {
                Ok(stopped) => stopped,
                Err(_) => {
                    eprintln!("Failed to get stopped status");
                    return;
                }
            };

            if *must_stop {
                println!("{color_red}STOPPED ACCEPT CONNECTIONS{style_reset}");
                drop(listener);

                handles.into_iter().for_each(|handle| _ = handle.join());
                return;
            }

            drop(must_stop);

            // listener is non-blocking, so it can check if the shard is stopped
            match listener.set_nonblocking(true) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Failed to set listener to non-blocking: {}", e);
                    return;
                }
            }

            match listener.accept() {
                Ok((stream, _)) => {
                    // Start listening for incoming messages in a thread
                    let shard_clone = shared_shard.clone();
                    let shareable_stream = Arc::new(Mutex::new(stream));
                    let stream_clone = Arc::clone(&shareable_stream);
                    let stopped_clone = stopped.clone();

                    let _handle = thread::spawn(move || {
                        println!("Inside thread spawn on accept_connections");
                        Shard::listen(&shard_clone, &stream_clone, stopped_clone);
                        println!("Listening thread finished");
                    });
                    handles.push(_handle);
                }
                Err(_) => {
                    // continue if there are no incoming connections
                }
            }
        }
    }

    // Listen for incoming messages
    pub fn listen(
        shared_shard: &Arc<Mutex<Shard>>,
        tcp_stream: &Arc<Mutex<TcpStream>>,
        stopped: Arc<Mutex<bool>>,
    ) {
        println!("Listening for incoming messages");

        let mut stream = match tcp_stream.lock() {
            Ok(stream) => stream,
            Err(_) => {
                eprintln!("Failed to get stream");
                return;
            }
        };

        match stream.set_nonblocking(true) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Failed to set stream to non-blocking: {e}");
                return;
            }
        }

        loop {
            // println!("Inside listen loop");
            let must_stop = match stopped.lock() {
                Ok(stopped) => stopped,
                Err(_) => {
                    eprintln!("Failed to get stopped status");
                    return;
                }
            };

            if *must_stop {
                println!("{color_red}STOPPED LISTENING{style_reset}");
                drop(stream);
                return;
            }

            drop(must_stop);

            // sleep for 1 millisecond to allow the stream to be ready to read
            thread::sleep(std::time::Duration::from_millis(1));
            let mut buffer = [0; 1024];

            match stream.set_nonblocking(true) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Failed to set stream to non-blocking: {e}");
                    return;
                }
            }

            // println!("Before stream read");
            match stream.read(&mut buffer) {
                Ok(chars) => {
                    if chars == 0 {
                        continue;
                    }

                    let message_string = String::from_utf8_lossy(&buffer);
                    let mut shard = match shared_shard.lock() {
                        Ok(shared_shard) => shared_shard,
                        Err(_) => {
                            eprintln!("Failed to get shared shard");
                            continue;
                        }
                    };

                    if message_string.is_empty() {
                        continue;
                    }

                    let message = match Message::from_string(&message_string) {
                        Ok(message) => message,
                        Err(e) => {
                            eprintln!(
                                "Failed to parse message: {e:?}. Message: [{message_string:?}]"
                            );
                            continue;
                        }
                    };

                    if shard.no_need_for_connection(message.to_owned()) {
                        return;
                    }

                    if let Some(response) = shard.get_response_message(message) {
                        println!(
                            "{color_bright_green}Received message: {message_string}{style_reset}"
                        );
                        println!("{color_bright_green}Sending response: {response}{style_reset}");

                        match stream.write_all(response.as_bytes()) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Failed to write response: {e}");
                            }
                        }
                    }
                }
                Err(_e) => {
                    // could not read from the stream, ignore
                }
            }
        }
    }

    // Shards may also receive a "HelloFromNode" message from other nodes. This message is used to establish a connection with the router.
    // So, if a shard receives a "HelloFromNode" message, it should not respond and the stream should be closed.
    fn no_need_for_connection(&self, message: Message) -> bool {
        message.get_message_type() == MessageType::HelloFromNode
    }

    fn get_response_message(&mut self, message: Message) -> Option<String> {
        match message.get_message_type() {
            MessageType::InitConnection => self.handle_init_connection_message(message),
            MessageType::AskMemoryUpdate => self.handle_memory_update_message(),
            MessageType::GetRouter => self.handle_get_router_message(),
            MessageType::HelloFromNode => None,
            _ => {
                eprintln!(
                    "Message type received: {:?}, not yet implemented",
                    message.get_message_type()
                );
                None
            }
        }
    }

    fn handle_init_connection_message(&mut self, message: Message) -> Option<String> {
        let router_info = message.get_data().node_info?;
        self.router_info = Arc::new(Mutex::new(Some(router_info.clone())));
        println!("{color_bright_green}Received an InitConnection message{style_reset}");
        let response_string = self.get_agreed_connection()?;
        Some(response_string)
    }

    fn handle_memory_update_message(&mut self) -> Option<String> {
        println!("{color_bright_green}Received an AskMemoryUpdate message{style_reset}");
        let response_string = self.get_memory_update_message()?;
        Some(response_string)
    }

    fn handle_get_router_message(&mut self) -> Option<String> {
        let self_clone = self.clone();
        let router_info: Option<NodeInfo> = {
            let router_info = match self_clone.router_info.as_ref().try_lock() {
                Ok(router_info) => router_info.clone(),
                Err(_) => {
                    eprintln!("Failed to get router info");
                    return None;
                }
            };
            router_info.clone()
        };

        if let Some(router_info) = router_info {
            let response_message = Message::new_router_id(router_info.clone());
            Some(response_message.to_string())
        } else {
            let response_message = Message::new_no_router_data();
            Some(response_message.to_string())
        }
    }

    fn get_agreed_connection(&self) -> Option<String> {
        let memory_manager = match self.memory_manager.as_ref().try_lock() {
            Ok(memory_manager) => memory_manager,
            Err(_) => {
                eprintln!("Failed to get memory manager");
                return None;
            }
        };
        let memory_percentage = memory_manager.available_memory_perc;
        let tables_max_id_clone = match self.tables_max_id.as_ref().try_lock() {
            Ok(tables_max_id) => tables_max_id.clone(),
            Err(_) => {
                eprintln!("Failed to get tables max id");
                return None;
            }
        };
        let response_message = Message::new_agreed(memory_percentage, tables_max_id_clone);

        Some(response_message.to_string())
    }

    fn get_memory_update_message(&mut self) -> Option<String> {
        match self.update() {
            Ok(()) => {
                println!("Memory updated successfully");
            }
            Err(e) => {
                eprintln!("Failed to update memory: {e:?}");
            }
        }
        let memory_manager = match self.memory_manager.as_ref().try_lock() {
            Ok(memory_manager) => memory_manager,
            Err(_) => {
                eprintln!("Failed to get memory manager");
                return None;
            }
        };
        let memory_percentage = memory_manager.available_memory_perc;
        let tables_max_id_clone = match self.tables_max_id.as_ref().try_lock() {
            Ok(tables_max_id) => tables_max_id.clone(),
            Err(_) => {
                eprintln!("Failed to get tables max id");
                return None;
            }
        };
        let response_message = Message::new_memory_update(memory_percentage, tables_max_id_clone);

        Some(response_message.to_string())
    }

    fn update(&mut self) -> Result<(), io::Error> {
        self.set_max_ids();
        match self.memory_manager.as_ref().try_lock() {
            Ok(mut memory_manager) => memory_manager.update(),
            Err(_) => {
                eprintln!("Failed to get memory manager");
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to get memory manager",
                ));
            }
        }
    }

    // Set the max ids for all tables in tables_max_id
    fn set_max_ids(&mut self) {
        let tables = self.get_all_tables_from_self(false);
        for table in tables {
            let query = format!("SELECT MAX(id) FROM {table}");
            if let Some(rows) = self.get_rows_for_query(&query) {
                let max_id: i32 = if let Ok(id) = rows[0].try_get(0) {
                    id
                } else {
                    eprintln!("Failed to get max id for table: {table}. Table might be empty",);
                    0
                };
                let mut tables_max_id = match self.tables_max_id.as_ref().try_lock() {
                    Ok(tables_max_id) => tables_max_id,
                    Err(_) => {
                        eprintln!("Failed to get tables max id");
                        return;
                    }
                };
                tables_max_id.insert(table, i64::from(max_id));
            }
        }
    }
}

impl NodeRole for Shard {
    fn backend(&self) -> Arc<Mutex<postgres::Client>> {
        self.backend.clone()
    }

    fn send_query(&mut self, query: &str) -> Option<String> {
        if query == "whoami;" {
            println!(
                "{color_bright_green}> I am Shard: {}:{}{style_reset}\n",
                self.ip, self.port
            );
            return None;
        }

        println!("{color_bright_green}Sending query to the shard database: ({query}){style_reset}");
        let rows = self.get_rows_for_query(query)?;
        let _ = self.update(); // Updates memory and tables_max_id
        Some(rows.convert_to_string())
    }

    fn stop(&mut self) {
        println!("{color_red}Stopping shard{style_reset}");
        match self.stopped.lock() {
            Ok(mut stopped) => {
                println!("{color_red}Setting stopped to true{style_reset}");
                *stopped = true;
            }
            Err(_) => {
                eprintln!("Failed to stop router");
            }
        }
        println!("{color_red}Shard stopped{style_reset}");
    }
}
