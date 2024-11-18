use super::router::Router;
use super::shard::Shard;
use crate::node::client::Client;
use crate::utils::node_config::get_nodes_config_raft;
use crate::utils::node_config::INIT_HISTORY_FILE_PATH;
use crate::utils::queries::print_rows;
use inline_colorization::*;
use postgres::Row;
use std::ffi::CStr;
use std::fmt::Error;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

use tokio::task;
use tokio::task::LocalSet;

pub trait NodeRole {
    fn backend(&self) -> Arc<Mutex<postgres::Client>>;
    /// Sends a query to the shard group
    fn send_query(&mut self, query: &str) -> Option<String>;
    fn stop(&mut self);

    fn get_all_tables(&mut self) -> Vec<String> {
        let query =
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'";
        let Some(rows) = self.get_rows_for_query(query) else {
            return Vec::new();
        };
        let mut tables = Vec::new();
        for row in rows {
            let table_name: String = row.get(0);
            tables.push(table_name);
        }
        tables
    }

    fn get_rows_for_query(&mut self, query: &str) -> Option<Vec<Row>> {
        // Código de error de SQLSTATE para "relation does not exist"
        const UNDEFINED_TABLE_CODE: &str = "42P01";

        match self
            .backend()
            .as_ref()
            .try_lock()
            .unwrap()
            .query(query, &[])
        {
            Ok(rows) => {
                print_rows(rows.clone());
                Some(rows)
            }
            Err(e) => {
                if let Some(db_error) = e.as_db_error() {
                    if db_error.code().code() == UNDEFINED_TABLE_CODE {
                        eprintln!("Failed to execute query: Relation (table) does not exist");
                    } else {
                        eprintln!("Failed to execute query: {e:?}");
                    }
                } else {
                    eprintln!("Failed to execute query: {e:?}");
                }
                None
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone)]
pub enum NodeType {
    Client,
    Router,
    Shard,
}

// MARK: Node Singleton

pub struct NodeInstance {
    pub instance: Option<Box<dyn NodeRole>>,
    pub ip: String,
    pub port: String,
    pub node_type: NodeType,
}

impl NodeInstance {
    fn new(instance: Box<dyn NodeRole>, ip: String, port: String, node_type: NodeType) -> Self {
        NodeInstance {
            instance: Some(instance),
            ip,
            port,
            node_type,
        }
    }
}

// MARK: Node Instance

pub static mut NODE_INSTANCE: Option<NodeInstance> = None;

pub fn get_node_instance() -> &'static mut NodeInstance {
    unsafe {
        NODE_INSTANCE
            .as_mut()
            .expect("Node instance not initialized")
    }
}

pub fn get_node_role() -> &'static mut dyn NodeRole {
    unsafe {
        NODE_INSTANCE
            .as_mut()
            .unwrap()
            .instance
            .as_mut()
            .unwrap()
            .as_mut()
    }
}

// MARK: PSQL use

/// External use of Node Instance from PostgreSQL
#[no_mangle]
pub extern "C" fn InitNodeInstance(node_type: NodeType, port: *const i8) {
    let found_port;
    unsafe {
        if port.is_null() {
            panic!("Received a null pointer for port");
        }

        let port_str = CStr::from_ptr(port);
        found_port = match port_str.to_str() {
            Ok(str) => str.to_string(),
            Err(_) => {
                panic!("Received an invalid UTF-8 string");
            }
        };
    }
    let ip = "127.0.0.1";
    new_node_instance(node_type.clone(), ip, &found_port);

    // If the node is a client, it does not need to run raft. Thus, it can return after initializing
    if node_type == NodeType::Client {
        return;
    }

    let (raft_transmitter, self_receiver): (Sender<bool>, Receiver<bool>) = mpsc::channel();
    let (self_transmitter, raft_receiver): (Sender<bool>, Receiver<bool>) = mpsc::channel();

    run_raft(ip.to_string(), found_port, raft_transmitter, raft_receiver);
    listen_raft_receiver(self_receiver, self_transmitter);
}

// MARK: Raft

fn run_raft(ip: String, port: String, transmitter: Sender<bool>, receiver: Receiver<bool>) {
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            task::block_in_place(|| {
                let local = LocalSet::new();
                local.block_on(&rt, async {
                    new_raft_instance(ip, port, transmitter, receiver).await;
                });
            });
        });
    });
}

async fn new_raft_instance(
    ip: String,
    port: String,
    transmitter: Sender<bool>,
    receiver: Receiver<bool>,
) {
    // Iterate over nodes and find the name of the one that matches ip and port:
    let nodes = get_nodes_config_raft();
    let node_id = nodes
        .nodes
        .iter()
        .find(|node| node.ip == ip && node.port == port)
        .expect("Could not find node in config file")
        .name
        .clone();

    let mut raft_module = raft::raft_module::RaftModule::new(
        node_id.clone(),
        ip.to_string(),
        port.parse::<usize>().unwrap(),
    );

    // This nevers comes back, unless the node is the last one in the config file
    raft_module
        .start(
            nodes,
            Some(&format!("{INIT_HISTORY_FILE_PATH}{}", node_id)),
            transmitter,
            receiver,
            true,
        )
        .await;
}

// MARK: Node Role

fn listen_raft_receiver(receiver: Receiver<bool>, transmitter: Sender<bool>) {
    thread::spawn(move || loop {
        match receiver.recv() {
            Ok(stopped) => {
                let role = if stopped {
                    NodeType::Router
                } else {
                    NodeType::Shard
                };
                match change_role(role.to_owned(), transmitter.clone()) {
                    Ok(_) => {
                        println!(
                            "{color_bright_green}Role changing finished succesfully{color_reset}"
                        );
                    }
                    Err(_) => {
                        println!("Error could not change role to {:?}", role);
                    }
                }
            }
            Err(_) => {
                // Do nothing
            }
        }
    });
}

fn change_role(new_role: NodeType, transmitter: Sender<bool>) -> Result<(), Error> {
    if new_role == NodeType::Client {
        eprintln!("NodeRole cannot be changed to Client, it is not a valid role");
        return Err(Error);
    };

    let node_instance = get_node_instance();
    let current_instance = &mut node_instance.instance;

    if node_instance.node_type == new_role {
        confirm_role_change(transmitter);
        return Ok(());
    }

    let ip = node_instance.ip.clone();
    let port = node_instance.port.clone();

    // Stop current instance
    match current_instance.as_mut() {
        Some(instance) => {
            instance.stop();
        }
        None => {
            eprintln!("Node instance not initialized");
            return Err(Error);
        }
    }

    match new_role {
        NodeType::Router => {
            init_router(&ip, &port);
        }
        NodeType::Shard => {
            init_shard(&ip, &port);
        }
        _ => {
            eprintln!("NodeRole can only be changed to Router or Shard");
            return Err(Error);
        }
    }

    confirm_role_change(transmitter);
    Ok(())
}

fn confirm_role_change(transmitter: Sender<bool>) {
    transmitter
        .send(true)
        .expect("Error sending true to raft transmitter");
}

fn new_node_instance(node_type: NodeType, ip: &str, port: &str) {
    // Initialize node based on node type
    match node_type {
        NodeType::Router => init_router(ip, port),
        NodeType::Shard => init_shard(ip, port),
        NodeType::Client => init_client(ip, port),
    }
}

fn init_router(ip: &str, port: &str) {
    let router = Router::new(ip, port);

    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(router.clone()),
            ip.to_string(),
            port.to_string(),
            NodeType::Router,
        ));
    }

    let shared_router: Arc<Mutex<Router>> = Arc::new(Mutex::new(router));
    let ip_clone = ip.to_string();
    let port_clone = port.to_string();
    let _handle = thread::spawn(move || {
        Router::wait_for_incomming_connections(&shared_router, ip_clone, port_clone);
    });
}

fn init_shard(ip: &str, port: &str) {
    let shard = Shard::new(ip, port);

    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(shard.clone()),
            ip.to_string(),
            port.to_string(),
            NodeType::Shard,
        ));
    }

    let shared_shard = Arc::new(Mutex::new(shard));
    let ip_clone = ip.to_string();
    let port_clone = port.to_string();
    let _handle = thread::spawn(move || {
        Shard::accept_connections(shared_shard, ip_clone, port_clone);
    });
}

fn init_client(ip: &str, port: &str) {
    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(Client::new(ip, port)),
            ip.to_string(),
            port.to_string(),
            NodeType::Client,
        ));
    }
}
