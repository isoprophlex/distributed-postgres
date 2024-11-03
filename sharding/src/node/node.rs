use crate::node::client::Client;
use crate::utils::node_config::get_nodes_config_raft;
use super::router::Router;
use super::shard::Shard;
use std::ffi::CStr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

use tokio::task;
use tokio::task::LocalSet;

pub trait NodeRole {
    /// Sends a query to the shard group
    fn send_query(&mut self, query: &str) -> Option<String>;
    fn stop(&mut self);
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum NodeType {
    Client,
    Router,
    Shard,
}

/* Node Singleton */

pub struct NodeInstance {
    pub instance: Option<Box<dyn NodeRole>>,
    pub ip: String,
    pub port: String,
    pub node_type: NodeType
}

impl NodeInstance {
    fn new(instance: Box<dyn NodeRole>, ip: String, port: String, node_type: NodeType) -> Self {
        NodeInstance {
            instance: Some(instance),
            ip,
            port,
            node_type
        }
    }
}

fn change_role(new_role: NodeType) {

    println!("Changing role to {:?}", new_role);
    
    if new_role == NodeType::Client {
        println!("NodeRole cannot be changed to Client");
        return;
    };

    println!("Trying to get node instance");
    let node_instance = get_node_instance();
    println!("AFTER get node instance");
    let current_instance = &mut node_instance.instance;

    println!("AFTER current instance");

    if node_instance.node_type == new_role {
        println!("NodeRole is already {:?}", new_role);
        return;
    }

    println!("node type changes");

    let ip = node_instance.ip.clone();
    let port = node_instance.port.clone();

    println!("ip: {}, port: {}", ip, port);

    // Stop current instance
    match current_instance.as_mut() {
        Some(instance) => {
            println!("Stopping current instance");
            instance.stop();
        }
        None => {
            panic!("Node instance not initialized");
        }
    }

    println!("AFTER STOPPING current instance");

    match new_role {
        NodeType::Router => {
            // TODO-A: Implement data migration to another shard
            let router = Router::new(&ip, &port);
            *current_instance = Some(Box::new(router));
            println!("NodeRole changed to Router");
        }
        NodeType::Shard => {
            let shard = Shard::new(&ip, &port);
            *current_instance = Some(Box::new(shard));
            println!("NodeRole changed to Shard");
        }
        _ => {
            panic!("NodeRole can only be changed to Router or Shard");
        }
    }

    println!("AFTER CHANGING current instance");
}

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

// External use of Node Instance
#[no_mangle]
pub extern "C" fn init_node_instance(
    node_type: NodeType,
    port: *const i8
) {
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
        println!("found_port: {}", found_port);
    }
    let ip = "127.0.0.1";
    println!("before init_shard ip: {}, port: {}", ip, found_port);
    new_node_instance(node_type, ip, &found_port);

    let (transmitter, receiver): (Sender<bool>, Receiver<bool>) = mpsc::channel();
    
    run_raft(ip.to_string(), found_port, transmitter);
    listen_receiver(receiver);
}

fn run_raft(ip: String, port: String, transmitter: Sender<bool>) {
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            task::block_in_place(|| {
                let local = LocalSet::new();
                local.block_on(&rt, async {
                    new_raft_instance(
                        ip,
                        port,
                        transmitter,
                    )
                    .await;
                });
            });
        });
    });
}

fn listen_receiver(receiver: Receiver<bool>) {
    thread::spawn(move || {
        loop {
            match receiver.recv() {
                Ok(stopped) => {
                    if stopped {
                        change_role(NodeType::Router);
                    }
                }
                Err(e) => {
                    println!("Error receiving from raft transmitter: {:?}", e);
                }
            }
        }
    });
}

fn new_node_instance(node_type: NodeType, ip: &str, port: &str) {
    // Initialize node based on node type
    match node_type {
        NodeType::Router => init_router(ip, port),
        NodeType::Shard => init_shard(ip, port),
        NodeType::Client => init_client(ip, port),
    }
}

async fn new_raft_instance(
    ip: String,
    port: String,
    transmitter: Sender<bool>,
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
        port.parse::<usize>().unwrap()
    );

    // This nevers comes back, unless the node is the last one in the config file
    // TODO-A here: check if needed, send "stopped" flag, so the raft module cant stop and come back if the instance is stopped
    raft_module
        .start(
            nodes,
            Some(&format!("../../../sharding/init_history/init_{}", node_id)),
            transmitter
        )
        .await;
}

fn init_router(ip: &str, port: &str) {
    let router = Router::new(ip, port);

    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(router.clone()),
            ip.to_string(),
            port.to_string(),
            NodeType::Router
        ));
    }

    let shared_router: Arc<Mutex<Router>> = Arc::new(Mutex::new(router));
    let ip_clone = ip.to_string();
    let port_clone = port.to_string();
    let _handle = thread::spawn(move || {
        Router::wait_for_client(&shared_router, &ip_clone, &port_clone);
        println!("Router comes back from wait_for_client");
    });

    println!("Router node initializes");
}

fn init_shard(ip: &str, port: &str) {
    println!("Sharding node initializing");
    let shard = Shard::new(ip, port);

    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(shard.clone()),
            ip.to_string(),
            port.to_string(),
            NodeType::Shard
        ));
    }

    let shared_shard = Arc::new(Mutex::new(shard));
    let ip_clone = ip.to_string();
    let port_clone = port.to_string();
    let _handle = thread::spawn(move || {
        Shard::accept_connections(&shared_shard, &ip_clone, &port_clone);
        println!("Shard comes back from accept_connections");
    });

    println!("Sharding node initializes");
}

fn init_client(ip: &str, port: &str) {
    println!("Client node initializing");
    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(Client::new(ip, port)),
            ip.to_string(),
            port.to_string(),
            NodeType::Client
        ));
    }
    println!("Client node initializes");
}
