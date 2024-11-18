use indexmap::IndexMap;
use postgres::{Client as PostgresClient, Row};
use rust_decimal::Decimal;
extern crate users;
use super::messages::node_info::find_name_for_node;
use super::node::NodeRole;
use super::send_query_result::SendQueryError;
use super::shard_manager::ShardManager;
use super::tables_id_info::TablesIdInfo;
use crate::node::messages::message::{Message, MessageType};
use crate::node::messages::node_info::NodeInfo;
use crate::node::send_query_result::{is_connection_closed, is_undefined_table};
use crate::utils::common::ConvertToString;
use crate::utils::common::{connect_to_node, Channel};
use crate::utils::node_config::{get_nodes_config, Node};
use crate::utils::queries::*;
use inline_colorization::*;
use std::fmt::Error;
use std::io::{Read, Write};
use std::sync::{Arc, MutexGuard, RwLock};
use std::usize;
use std::{io, net::TcpListener, net::TcpStream, sync::Mutex, thread};

/// This struct represents the Router node in the distributed system. It has the responsibility of routing the queries to the appropriate shard or shards.
#[repr(C)]
#[derive(Clone)]
pub struct Router {
    ///  `IndexMap`:
    ///     `key`: shardId
    ///     `value`: Shard's Client
    shards: Arc<Mutex<IndexMap<String, PostgresClient>>>,
    shard_manager: Arc<ShardManager>,
    ///  `IndexMap`:
    ///     `key`: Hash
    ///     `value`: shardId
    comm_channels: Arc<RwLock<IndexMap<String, Channel>>>,
    ip: Arc<str>,
    port: Arc<str>,
    name: String,
    pub stopped: Arc<Mutex<bool>>,
    /// Backend for the router, only used when redistributing data
    backend: Arc<Mutex<PostgresClient>>,
}

impl Router {
    /// Creates a new Router node with the given port and ip, connecting it to the shards specified in the configuration file.
    pub fn new(ip: &str, port: &str) -> Option<Self> {
        println!("Inside new::Router");
        Router::initialize_router_with_connections(ip, port)
    }

    /// Listen for incoming connections from clients or new shards.
    pub fn wait_for_incoming_connections(
        shared_router: &Arc<Mutex<Router>>,
        ip: String,
        waiting_port: String,
    ) {

        let port_number = match waiting_port.parse::<u64>() {
            Ok(port) => port,
            Err(_) => {
                eprintln!("Failed to parse port number");
                return;
            }
        };
        let port = port_number + 1000;
        println!("Attempting to bind listener to port: {}", port);

        let listener = match TcpListener::bind(format!("{}:{}", ip, port)) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind listener to port: {}", e);
                return;
            }
        };

        println!("wait_for_incoming_connections");

        loop {
            let stopped = {
                let router = match shared_router.lock() {
                    Ok(shared_router) => shared_router,
                    Err(_) => {
                        eprintln!("Failed to get shared router");
                        return;
                    }
                };

                let router_lock = match router.stopped.lock() {
                    Ok(stopped) => *stopped,
                    Err(_) => {
                        eprintln!("Failed to get stopped status");
                        return;
                    }
                };
                router_lock
            };

            if stopped {
                println!("Stopped is true");
                drop(listener);
                return;
            }

            if let Err(e) = listener.set_nonblocking(true) {
                eprintln!("Failed to set listener to non-blocking: {}", e);
                return;
            }

            match listener.accept() {
                Ok((stream, addr)) => {
                    println!(
                        "{color_bright_green}[ROUTER] New connection accepted from {addr}.{style_reset}"
                    );

                    let router_clone = shared_router.clone();
                    let shareable_stream = Arc::new(Mutex::new(stream));
                    let stream_clone = Arc::clone(&shareable_stream);

                    thread::spawn(move || {
                        println!("Inside thread in wait_for_client");
                        Router::listen(&router_clone, &stream_clone);
                    });
                }
                Err(e) => {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        eprintln!("Failed to accept connection: {}", e);
                    }
                }
            }
        }
    }

    // Listen for incoming messages
    pub fn listen(shared_router: &Arc<Mutex<Router>>, stream: &Arc<Mutex<TcpStream>>) {
        loop {
            // Loscks the router to check the 'stopped' status
            let stopped = {
                let router = match shared_router.lock() {
                    Ok(router) => router,
                    Err(_) => {
                        eprintln!("Failed to get shared router");
                        return;
                    }
                };

                // Checks the 'stopped' status and releases the router lock
                let router_clone = match router.stopped.lock() {
                    Ok(stopped) => *stopped,
                    Err(_) => {
                        eprintln!("Failed to get stopped status");
                        continue;
                    }
                };
                router_clone
            };

            if stopped {
                println!("Stopped is true");
                return;
            }

            // Waits briefly to allow the stream to be ready for reading
            thread::sleep(std::time::Duration::from_millis(1));
            let mut buffer = [0; 1024];

            // Locks the stream and sets the read timeout
            let mut stream = match stream.lock() {
                Ok(stream) => stream,
                Err(_) => {
                    eprintln!("Failed to lock stream");
                    continue;
                }
            };

            if let Err(_) = stream.set_read_timeout(Some(std::time::Duration::new(10, 0))) {
                continue;
            }

            match stream.read(&mut buffer) {
                Ok(chars) => {
                    if chars == 0 {
                        continue;
                    }

                    // Parses the buffer to a String
                    let message_string = String::from_utf8_lossy(&buffer);

                    // Locks the router to get and send the response
                    let response = {
                        let mut router = match shared_router.lock() {
                            Ok(router) => router,
                            Err(_) => {
                                eprintln!("Failed to get shared router");
                                continue;
                            }
                        };
                        router.get_response_message(&message_string).clone()
                    };

                    if let Some(response) = response {
                        // Sends the response through the stream (already locked)
                        if let Err(e) = stream.write_all(response.as_bytes()) {
                            eprintln!("Failed to send response: {:?}", e);
                        }
                    }
                }
                Err(_) => {
                    // Could not read from stream, continue
                }
            }
        }
    }
    fn get_response_message(&mut self, message: &str) -> Option<String> {
        if message.is_empty() {
            return None;
        }

        let message = match Message::from_string(message) {
            Ok(message) => message,
            Err(_) => {
                return None;
            }
        };

        match message.get_message_type() {
            MessageType::Query => self.handle_query_message(&message),
            MessageType::GetRouter => self.handle_get_router_message(),
            MessageType::HelloFromNode => self.handle_hello_from_node_message(&message),
            _ => {
                eprintln!(
                    "Message type received: {:?}, not yet implemented",
                    message.get_message_type()
                );
                None
            }
        }
    }
    
    fn handle_query_message(&mut self, message: &Message) -> Option<String> {
        let query = match message.get_data().query {
            Some(query) => query,
            None => {
                eprintln!("Failed to get query from message");
                return None;
            }
        };

        let Some(response) = self.send_query(&query) else {
            eprintln!("Failed to send query to shards");
            let response_message = Message::new_query_response(
                "[⚠️] There are no nodes available at this moment. Please try again later."
                    .to_string(),
            );
            return Some(response_message.to_string());
        };
        let response_message = Message::new_query_response(response);
        Some(response_message.to_string())
    }

    fn handle_get_router_message(&mut self) -> Option<String> {
        let self_clone = self.clone();
        let ip = self_clone.ip.clone().to_string();
        let port = self_clone.port.clone().to_string();
        let name = self_clone.name.clone().to_string();
        let router_info: NodeInfo = NodeInfo { ip, port, name };

        let response_message = Message::new_router_id(router_info.clone());
        Some(response_message.to_string())
    }

    fn handle_hello_from_node_message(&mut self, message: &Message) -> Option<String> {
        println!("Received HelloFromNode message");

        let node_info = match message.get_data().node_info {
            Some(node_info) => node_info,
            None => {
                eprintln!("Failed to get node info from message");
                return None;
            }
        };

        self.configure_shard_connection_to(Node {
            ip: node_info.ip,
            port: node_info.port.clone(),
            name: node_info.name,
        });

        self.duplicate_tables_into(&node_info.port);
        if self.shard_manager.count() == 1 {
            self.redistribute_data();
        }
        Some("OK".to_string())
    }

    fn duplicate_tables_into(&mut self, shard_id: &str) {
        let tables = self.get_all_tables_from_shards();

        println!("Tables: {tables:?}");

        for table in tables {
            let create_query = self.generate_create_table_query(&table, None);
            if !create_query.is_empty() {
                _ = self.send_query_to_shard(shard_id, &create_query, true);
            }
        }
    }

    fn get_all_tables_from_shards(&mut self) -> Vec<String> {
        self.shard_manager.get_table_names_for_all()
    }

    /// Initializes the Router node with connections to the shards specified in the configuration file.
    fn initialize_router_with_connections(ip: &str, port: &str) -> Option<Router> {
        let shards: IndexMap<String, PostgresClient> = IndexMap::new();
        let comm_channels: IndexMap<String, Channel> = IndexMap::new();
        let shard_manager = ShardManager::new();

        let backend = match connect_to_node(ip, port) {
            Ok(backend) => backend,
            Err(_) => {
                eprintln!("Failed to connect to backend");
                return None;
            }
        };

        let name = match find_name_for_node(ip.to_string(), port.to_string()) {
            Some(name) => name,
            None => {
                eprintln!("Failed to find name for node. Using ip and port for identification");
                format!("{}:{}", ip, port)
            }
        };

        let mut router = Router {
            shards: Arc::new(Mutex::new(shards)),
            shard_manager: Arc::new(shard_manager),
            comm_channels: Arc::new(RwLock::new(comm_channels)),
            ip: Arc::from(ip),
            port: Arc::from(port),
            name,
            stopped: Arc::new(Mutex::new(false)),
            backend: Arc::new(Mutex::new(backend)),
        };

        router.configure_connections();
        router.redistribute_data();
        Some(router)
    }

    fn configure_connections(&mut self) {
        let config = get_nodes_config();
        for shard in config.nodes {
            println!("Configuring connection to shard: {:?}", shard);
            if (shard.ip == self.ip.as_ref()) && (shard.port == self.port.as_ref()) {
                self.name = shard.name.clone();
                continue;
            }
            self.configure_shard_connection_to(shard);
        }
    }

    /// Configures the connection to a shard with the given ip and port.
    pub fn configure_shard_connection_to(&mut self, node: Node) {
        let node_ip = node.ip;
        let node_port = node.port;

        if self
            .set_health_connection(node_ip.as_str(), node_port.as_str())
            .is_err()
        {
            println!("Failed to connect to node: {}", node.name);
            return;
        }

        println!("Connecting to ip {} and port: {}", node_ip, node_port);

        let Ok(shard_client) = connect_to_node(&node_ip, &node_port) else {
            println!("Failed to connect to port: {node_port}");
            return;
        };

        println!("CONNECTED to ip {} and port: {}", node_ip, node_port);

        self.save_shard_client(node_port.to_string(), shard_client);
    }

    /// Saves the shard client in the Router's shards `IndexMap` with its corresponding shard id as key.
    fn save_shard_client(&mut self, shard_id: String, shard_client: PostgresClient) {
        let mut shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                eprintln!("Failed to get shards lock");
                return;
            }
        };
        shards.insert(shard_id, shard_client);
    }

    /// Sets the health_connection to the shard with the given ip and port, initializing the communication with a handshake between the router and the shard.
    fn set_health_connection(&mut self, node_ip: &str, node_port: &str) -> Result<(), Error> {
        let Ok(health_connection) = Router::get_shard_channel(node_ip, node_port) else {
            println!("Failed to create health-connection to port: {node_port}");
            return Err(Error);
        };

        if self.send_init_connection_message(&health_connection.clone(), node_port) {
            self.save_comm_channel(node_port.to_string(), health_connection);
        }
        Ok(())
    }

    /// Saves the communication channel to the shard with the given shard id as key.
    fn save_comm_channel(&mut self, shard_id: String, channel: Channel) {
        let mut comm_channels = match self.comm_channels.write() {
            Ok(comm_channels) => comm_channels,
            Err(_) => {
                eprintln!("Failed to get comm channels lock");
                return;
            }
        };
        comm_channels.insert(shard_id, channel);
    }

    /// Sends the `InitConnection` message to the shard with the given shard id, initializing the communication with a handshake between the router and the shard.
    /// The shard will respond with a `MemoryUpdate` message, which will be handled by the router updating the shard's memory size in the `ShardManager`.
    fn send_init_connection_message(
        &mut self,
        health_connection: &Channel,
        node_port: &str,
    ) -> bool {
        // Send InitConnection Message to Shard and save shard to ShardManager
        let mut stream = match health_connection.stream.as_ref().lock() {
            Ok(stream) => stream,
            Err(_) => {
                eprintln!("Failed to get stream lock");
                return false;
            }
        };

        let node_info = NodeInfo {
            ip: self.ip.as_ref().to_string(),
            port: self.port.as_ref().to_string(),
            name: self.name.clone(),
        };
        let update_message = Message::new_init_connection(node_info);
        println!("Sending message to shard: {update_message:?}");

        let message_string = update_message.to_string();
        match stream.write_all(message_string.as_bytes()) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("Failed to write message to shard");
                return false;
            }
        }

        println!("Waiting for response from shard");

        let response: &mut [u8] = &mut [0; 1024];

        // Wait for timeout and read response
        match stream.set_read_timeout(Some(std::time::Duration::new(10, 0))) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("Failed to set read timeout");
                return false;
            }
        };

        if stream.read(response).is_ok() {
            let response_string = String::from_utf8_lossy(response);
            let Ok(response_message) = Message::from_string(&response_string) else {
                eprintln!("Failed to parse message from shard");
                return false;
            };

            return self.handle_response(&response_message, node_port);
        }
        println!("{color_red}Shard {node_port} did not respond{style_reset}");
        false
    }

    /// Handles the responses from the shard from the `health_connection` channel.
    fn handle_response(&mut self, response_message: &Message, node_port: &str) -> bool {
        match response_message.get_message_type() {
            MessageType::Agreed => self.handle_agreed_message(&response_message.clone(), node_port),
            MessageType::MemoryUpdate => {
                self.handle_memory_update_message(&response_message.clone(), node_port)
            }
            _ => {
                println!("{color_red}Shard {node_port} denied the connection{style_reset}");
                false
            }
        }
    }

    fn handle_agreed_message(&mut self, message: &Message, node_port: &str) -> bool {
        println!("{color_bright_green}Shard {node_port} accepted the connection{style_reset}");

        let payload = match message.get_data().payload {
            Some(payload) => payload,
            None => {
                eprintln!("Failed to get payload from message");
                return false;
            }
        };
        let memory_size = payload;

        let max_ids_info = match message.get_data().max_ids {
            Some(max_ids_info) => max_ids_info,
            None => {
                eprintln!("Failed to get max ids info from message");
                return false;
            }
        };
        let max_ids_info = max_ids_info;

        println!("{color_bright_green}Memory size: {memory_size}{style_reset}");
        println!("{color_bright_green}Max Ids for Shard: {max_ids_info:?}{style_reset}");
        self.save_shard_in_manager(memory_size, node_port, max_ids_info);
        true
    }

    fn handle_memory_update_message(&mut self, message: &Message, node_port: &str) -> bool {
        let payload = match message.get_data().payload {
            Some(payload) => payload,
            None => {
                eprintln!("Failed to get payload from message");
                return false;
            }
        };
        let memory_size = payload;
        
        let max_ids_info = match message.get_data().max_ids {
            Some(max_ids_info) => max_ids_info,
            None => {
                eprintln!("Failed to get max ids info from message");
                return false;
            }
        };
        let max_ids_info = max_ids_info;

        println!(
            "{color_bright_green}Shard {node_port} updated its memory size to {memory_size}{style_reset}"
        );
        println!("{color_bright_green}Max Ids for Shard: {max_ids_info:?}{style_reset}");
        self.update_shard_in_manager(memory_size, node_port, max_ids_info);
        true
    }

    /// Adds a shard to the `ShardManager` with the given memory size and shard id.
    fn save_shard_in_manager(&mut self, memory_size: f64, shard_id: &str, max_ids: TablesIdInfo) {
        let mut shard_manager = self.shard_manager.as_ref().clone();
        shard_manager.add_shard(memory_size, shard_id.to_string());
        shard_manager.save_max_ids_for_shard(shard_id.to_string(), max_ids);
        println!("{color_bright_green}Shard {shard_id} added to ShardManager{style_reset}");
        println!("Shard Manager: {shard_manager:?}");
    }

    /// Updates the shard in the `ShardManager` with the given memory size and shard id.
    fn update_shard_in_manager(&mut self, memory_size: f64, shard_id: &str, max_ids: TablesIdInfo) {
        let mut shard_manager = self.shard_manager.as_ref().clone();
        shard_manager.update_shard_memory(memory_size, shard_id.to_string());
        shard_manager.save_max_ids_for_shard(shard_id.to_string(), max_ids);
        println!("{color_bright_green}Shard {shard_id} updated in ShardManager{style_reset}");
        println!("Shard Manager: {shard_manager:?}");
    }

    /// Establishes a health connection with the node with the given ip and port, returning a Channel.
    fn get_shard_channel(node_ip: &str, node_port: &str) -> Result<Channel, io::Error> {

        let port_number = match node_port.parse::<u64>() {
            Ok(port) => port,
            Err(_) => {
                eprintln!("Failed to parse port number");
                return Err(io::Error::new(io::ErrorKind::Other, "Failed to parse port number"));
            }
        };

        let port = port_number + 1000;
        println!("Attempting to connect to port: {}", port);
        match TcpStream::connect(format!("{node_ip}:{port}")) {
            Ok(stream) => {
                println!(
                    "{color_bright_green}Health connection established with {node_ip}:{port}{style_reset}"
                );
                Ok(Channel {
                    stream: Arc::new(Mutex::new(stream)),
                })
            }
            Err(e) => {
                println!(
                    "{color_red}Error establishing health connection with {node_ip}:{port}. Error: {e:?}{style_reset}"
                );
                Err(e)
            }
        }
    }

    /// Function that receives a query and checks for shards with corresponding data.
    /// If the query is an INSERT query, it will return the specific shard that the query should be sent to.
    /// If the query is not an INSERT query, it will return all shards.
    /// The second return value is a boolean that indicates if the shards need to update their memory after the query is executed. This will be true if the query affects the memory state of the system.
    /// Returns the query formatted if needed (if there's a 'WHERE ID=' clause, offset might need to be removed)
    fn get_data_needed_from(&mut self, query: &str) -> (Vec<String>, bool, String) {
        if let Some(id) = get_id_if_exists(query) {
            return self.get_specific_shard_with(id, query);
        }

        if query_is_insert(query) {
            let shard = match self.shard_manager.peek() {
                Some(shard) => shard,
                None => {
                    return ([].to_vec(), false, query.to_string());
                }
            };
            (vec![shard.clone()], true, query.to_string())
        } else {
            let shards = match self.shards.lock() {
                Ok(shards) => shards,
                Err(_) => {
                    eprintln!("Failed to get shards");
                    return ([].to_vec(), false, query.to_string());
                }
            };

            // Return all shards
            (
                shards.keys().cloned().collect(),
                query_affects_memory_state(query),
                query.to_string(),
            )
        }
    }

    fn get_specific_shard_with(&mut self, mut id: i64, query: &str) -> (Vec<String>, bool, String) {
        let shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                eprintln!("Failed to get shards");
                return ([].to_vec(), false, query.to_string());
            }
        };

        let Some(table_name) = get_table_name_from_query(query) else {
            return (
                shards.keys().cloned().collect(),
                query_affects_memory_state(query),
                query.to_string(),
            );
        };

        println!("Table name: {table_name}");

        for shard_id in shards.keys() {
            let Some(max_id) = self
                .shard_manager
                .get_max_ids_for_shard_table(shard_id, &table_name)
            else {
                continue;
            };

            if id > max_id {
                id -= max_id;
            } else {
                let formatted_query = format_query_with_new_id(query, id);
                return (
                    vec![shard_id.clone()],
                    query_affects_memory_state(query),
                    formatted_query,
                );
            }
        }

        println!("ID not found in any shard");
        (
            shards.keys().cloned().collect(),
            query_affects_memory_state(query),
            query.to_string(),
        )
    }

    fn format_response(&self, shards_responses: IndexMap<String, Vec<Row>>, query: &str) -> Option<String> {
        let Some(table_name) = get_table_name_from_query(query) else {
            eprintln!("Failed to get table name from query");
            return None;
        };

        let mut rows_offset: Vec<(Vec<Row>, i64)> = Vec::new();
        let mut last_offset: i64 = 0;
        for (shard_id, rows) in shards_responses {
            let Some(offset) = self
                .shard_manager
                .get_max_ids_for_shard_table(&shard_id, &table_name)
            else {
                // No offset to be formatted
                return Some(rows.convert_to_string());
            };
            rows_offset.push((rows, last_offset));
            last_offset = offset;
        }

        format_rows_with_offset(rows_offset)
    }
}

// MARK: - NodeRole implementation
impl NodeRole for Router {
    fn backend(&self) -> Arc<Mutex<postgres::Client>> {
        self.backend.clone()
    }

    fn send_query(&mut self, received_query: &str) -> Option<String> {
        if received_query == "whoami;" {
            println!("> I am Router: {}:{}\n", self.ip, self.port);
            return None;
        }

        println!("Router send_query called with query: {received_query:?}");

        let (shards, affects_memory, query) = self.get_data_needed_from(received_query);

        // If there are no shards available, the router uses its own backend to execute the query and return the response.
        if shards.is_empty() {
            return match self.send_query_to_backend(received_query) {
                Some(response) => Some(response),
                None => {
                    return None;
                }
            };
        }

        // If the router suddenly is alone, it's going to have to hold on to the data until the shards are available again. So the tables must be available in its backend too.
        if query_is_create_or_drop(&received_query) {
            _ = self.send_query_to_backend(received_query);
        }

        type ShardResponses = Arc<Mutex<IndexMap<String, Vec<Row>>>>;

        let shards_responses: ShardResponses = Arc::new(Mutex::new(IndexMap::new()));
        let rows = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();

        for shard_id in shards.clone() {
            let mut self_clone = self.clone();
            let query_clone = query.clone();
            let rows_clone = rows.clone();
            let shards_repsonses_clone = shards_responses.clone();

            let _shard_response_handle = thread::spawn(move || -> Result<(), (SendQueryError, Option<String>)> {
                let shard_response = match self_clone.send_query_to_shard(&shard_id, &query_clone, affects_memory) {
                        Ok(rows) => rows,
                        Err(send_query_error) => {
                            return Err(send_query_error);
                        }
                    };

                if !shard_response.is_empty() {
                    let mut shards_responses = match shards_repsonses_clone.lock() {
                        Ok(shards_responses) => shards_responses,
                        Err(_) => {
                            eprintln!("Failed to get shards responses lock");
                            return Err((SendQueryError::Other("Failed to get shards responses lock".to_string()), None));
                        }
                    };

                    shards_responses.insert(shard_id, shard_response.clone());

                    let mut rows_lock = match rows_clone.lock() {
                        Ok(rows) => rows,
                        Err(_) => {
                            eprintln!("Failed to get rows lock");
                            return Err((SendQueryError::Other("Failed to get rows lock".to_string()), None));
                        }
                    };
                    rows_lock.extend(shard_response);
                }
                return Ok(());
            });
            handles.push(_shard_response_handle);
        }

        let mut table_errors = 0;
        // Wait for all threads to finish
        for handle in handles {
            match handle.join() {
                Ok(result) => {
                    match result {
                        Ok(_) => {},
                        Err((err, shard_id)) => {
                            table_errors += if err == SendQueryError::UndefinedTable {1} else {0};
                            let client_was_closed = err == SendQueryError::ClientIsClosed;
                            let shards_count = shards.len();

                            if let Some(shard_id) = shard_id {
                                // If the shard is closed, the router will remove it from the shards list
                                if client_was_closed {
                                    self.delete_shard(&shard_id);
                                }
                            }

                            // If there was only one shard and it was closed, the router will execute the query itself
                            if shards_count == 1 && client_was_closed {
                                return match self.send_query_to_backend(&query) {
                                    Some(response) => Some(response),
                                    None => {
                                        return None;
                                    }
                                };
                            }
                        },
                    }
                }
                Err(_) => {
                    // The thread panicked
                    println!("Thread panicked");
                }
            };
        }

        if table_errors == shards.len() {
            return Some("Relation (table) does not exist".to_string());
        }

        println!("All threads finished");
        let responses = match shards_responses.lock() {
            Ok(shards_responses) => shards_responses.clone(),
            Err(_) => {
                eprintln!("Failed to get shards responses lock");
                return None;
            }
        };

        let response = if query_is_select(&query) && !responses.is_empty() {
            match self.format_response(responses, &query) {
                Some(response) => response,
                None => {
                    eprintln!("Failed to format response");
                    return None;
                }
            }
        } else {
            let rows_lock = match rows.lock() {
                Ok(rows) => rows,
                Err(_) => {
                    eprintln!("Failed to get rows lock");
                    return None;
                }
            };
            // Query is not select or shard response is empty.
            // Therefore, there's no need to format the response to abstract the offset
            rows_lock.convert_to_string()
        };

        print_query_response(response.clone());
        Some(response)
    }

    fn stop(&mut self) {
        match self.stopped.lock() {
            Ok(mut stopped) => {
                *stopped = true;
            }
            Err(_) => {
                eprintln!("Failed to stop router");
            }
        }
    }
}

// MARK: - Communication with shards
impl Router {
    fn get_stream(&self, shard_id: &str) -> Option<Arc<Mutex<TcpStream>>> {
        let Ok(comm_channels) = self.comm_channels.read() else {
            eprintln!("Failed to get comm channels");
            return None;
        };

        let Some(shard_comm_channel) = comm_channels.get(&shard_id.to_string()) else {
            eprintln!("Failed to get comm channel for shard {shard_id}");
            return None;
        };

        Some(shard_comm_channel.stream.clone())
    }
    
    fn init_message_exchange(
        &mut self,
        message: &Message,
        writable_stream: &mut MutexGuard<TcpStream>,
        shard_id: &str,
    ) -> bool {
        match writable_stream.write_all(message.to_string().as_bytes()) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("Failed to write message to shard {shard_id}");
                return false;
            }
        };  
        let mut response: [u8; 1024] = [0; 1024];

        // Read and handle message
        match writable_stream.read(&mut response) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("Failed to read message from shard {shard_id}");
                return false;
            }
        };
        let response_string = String::from_utf8_lossy(&response);

        let Ok(response_message) = Message::from_string(&response_string) else {
            eprintln!("Failed to parse message from shard");
            return false;
        };

        self.handle_response(&response_message, shard_id)
    }

    /// Sends a message to the shard asking for a memory update.
    /// This must be called each time a query is sent which affects the memory (see `query_affects_memory_state` at queries.rs), and may be used to update the shard's memory size in the `ShardManager` in other circumstances.
    fn ask_for_memory_update(&mut self, shard_id: &str) {
        let Some(stream) = self.get_stream(shard_id) else {
            eprintln!("Failed to get stream for shard {shard_id}");
            return;
        };

        let Ok(mut writable_stream) = stream.try_lock() else {
            eprintln!("Failed to get writable stream for shard {shard_id}");
            return;
        };

        // Write message
        let message = Message::new_ask_memory_update();
        self.init_message_exchange(&message, &mut writable_stream, shard_id);
    }

    fn send_query_to_shard(&mut self, shard_id: &str, query: &str, update: bool) -> Result<Vec<Row>, (SendQueryError,Option<String>)> {
        let self_clone = self.clone();

        let mut shards = match self_clone.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                eprintln!("Failed to get shards");
                return Err((SendQueryError::Other("Failed to get shards".to_string()), None));
            }
        };

        println!("Sending query to shard {shard_id}: {query}");
        if let Some(shard) = shards.get_mut(shard_id) {
            let rows = match shard.query(query, &[]) {
                Ok(rows) => rows,
                Err(error) => {
                    eprintln!("Failed to send the query to the shard: ");
                    if is_undefined_table(&error) {
                        eprint!("Relation (table) does not exist\n");
                        return Err((SendQueryError::UndefinedTable, None));
                    } else if is_connection_closed(&error) {
                        println!("Connection closed with shard {shard_id}");
                        return Err((SendQueryError::ClientIsClosed, Some(shard_id.to_string())));
                    } else {
                        eprint!("{error:?}\n");
                    }
                    return Err((SendQueryError::Other(format!("{error:?}")), None));
                }
            };

            if update {
                self.ask_for_memory_update(shard_id);
            }

            return Ok(rows);
        }
        eprintln!("Shard {shard_id:?} not found");
        Err((SendQueryError::Other("Shard not found".to_string()), None))
    }

    fn send_query_to_backend(&mut self, query: &str) -> Option<String> {
        println!(
            "{color_bright_green}Sending query to the router database: ({query}){style_reset}"
        );
        let rows = self.get_rows_for_query(query)?;
        let response = format_rows_without_offset(rows);
        Some(response)
    }

    fn delete_shard(&mut self, shard_id: &str) {
        println!("Deleting shard {shard_id}");
        let mut shard_manager = self.shard_manager.as_ref().clone();

        shard_manager.delete(shard_id.to_string());
        let mut shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                eprintln!("Failed to get shards");
                return;
            }
        };
        shards.swap_remove(shard_id);
    }
}

// MARK: - Data Redistribution
impl Router {
    fn backend_has_data(&mut self) -> bool {
        let query = "SELECT * FROM information_schema.tables WHERE table_schema = 'public'";
        let rows = self.get_rows_for_query(query);
        rows.is_some()
    }

    fn redistribute_data(&mut self) {
        if !self.backend_has_data() {
            println!("No data found in backend. Skipping redistribution.");
            return;
        }

        let shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                eprintln!("Failed to get shards");
                return;
            }
        };

        if shards.is_empty() {
            println!("No shards found to redistribute data. Holding on to data until shards are available.");
            return;
        }

        // drop shard lock
        drop(shards);

        let tables = self.get_all_tables_from_self();

        // Prepare data structures
        let mut starting_queries = Vec::new();

        for table in &tables {
            // Generate CREATE TABLE query
            let create_query = self.generate_create_table_query(table, None);
            if !create_query.is_empty() {
                starting_queries.push(create_query);
            }

            let mut insert_queries = Vec::new();
            // Fetch all rows from the table
            if let Some(rows) = self.get_rows_for_query(&format!("SELECT * FROM {}", table)) {
                // Convert each row to an INSERT query and store it
                for row in rows {
                    let insert_query = self.row_to_insert_query(&row, table);
                    insert_queries.push(insert_query);
                }
            }

            // Send queries to appropriate shards
            for insert_query in &insert_queries {
                let (shards, _, formatted_query) = self.get_data_needed_from(insert_query);

                for shard_id in shards {
                    // Send `starting_query` if it hasn't been sent for this table
                    let table_starting_query =
                        match starting_queries.iter().find(|q| q.contains(table)) {
                            Some(q) => q,
                            None => {
                                continue;
                            }
                        };
                    _ = self.send_query_to_shard(&shard_id, table_starting_query, false);

                    // Send the actual insert query
                    _ = self.send_query_to_shard(&shard_id, &formatted_query, true);
                }
            }
        }
        // Drop all tables from router backend
        self.empty_tables(&tables);
    }

    fn generate_create_table_query(&mut self, table: &str, shard_id: Option<String>) -> String {
        // Dynamically generate CREATE TABLE statement for the specified table, getting the column names and data types from the information_schema.columns table
        let query = format!(
            "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = '{}'",
            table
        );

        println!("Query: {query}");

        let rows = if let Some(shard) = shard_id {
            println!("Sending query to shard {shard}: {query}");
            match self.send_query_to_shard(&shard, &query, false) {
                Ok(rows) => rows,
                Err(_) => {
                    eprintln!("Failed to get rows for query");
                    Vec::new()
                }
            }
        } else {
            match self.get_rows_for_query(&query) {
                Some(rows) => rows,
                None => {
                    eprintln!("Failed to get rows for query");
                    Vec::new()
                }
            }
        };

        println!("Rows: {rows:?}");

        let mut columns_definitions: Vec<String> = rows
            .iter()
            .enumerate()
            .map(|(_, row)| {
                let column_name: String = row.get("column_name");
                // PostgresClient does not support getting the PrimaryKey, so all tables will have a SERIAL PRIMARY KEY called "id". If you want to fix this, be my guest
                let data_type: String = if column_name == "id" {
                    "SERIAL PRIMARY KEY".to_string()
                } else {
                    row.get("data_type")
                };
                format!("{} {}", column_name, data_type)
            })
            .collect();

        // sort columns_definitions
        columns_definitions.sort();

        format!(
            "CREATE TABLE IF NOT EXISTS {} ({})",
            table,
            columns_definitions.join(", ")
        )
    }

    fn row_to_insert_query(&self, row: &Row, table: &str) -> String {
        let columns = row.columns();
        let column_names: Vec<String> = columns
            .iter()
            .skip(1)
            .map(|c| c.name().to_string())
            .collect();

        // Convert each column value to a string using your ConvertToString trait
        let mut result: Vec<String> = vec![];

        for (i, _) in row.columns().iter().enumerate() {
            // First column is the ID, we skip it
            if i == 0 {
                continue;
            }
            // Try to get the value as a String, If it fails, try to get it as an i32. Same for f64 and Decimal
            let formatted_value = match row.try_get::<usize, String>(i) {
                Ok(v) => format!("'{}'", v),
                Err(_) => match row.try_get::<usize, i32>(i) {
                    Ok(v) => format!("{}", v),
                    Err(_) => match row.try_get::<usize, f64>(i) {
                        Ok(v) => format!("{}", v),
                        Err(_) => match row.try_get::<usize, Decimal>(i) {
                            Ok(v) => format!("{}", v),
                            Err(_) => String::new(),
                        },
                    },
                },
            };

            result.push(formatted_value);
        }

        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            column_names.join(", "),
            result.join(", ")
        );

        println!("{color_bright_green}Query: {query:?}{style_reset}");

        query
    }

    fn empty_tables(&mut self, tables: &[String]) {
        for table in tables {
            let drop_query = format!("DELETE * FROM {}", table);
            let _ = self.get_rows_for_query(&drop_query);
            println!("Table {} was emptied", table);
        }
    }
}
