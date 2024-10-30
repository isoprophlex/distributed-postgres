use crate::node::client::Client;
use crate::utils::node_config::get_nodes_config_raft;

use super::router::Router;
use super::shard::Shard;
use actix_rt::System;
use futures::executor::block_on;
use raft::raft_module::{self, RaftModule};
use std::ffi::CStr;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

use tokio::task;
use tokio::task::LocalSet;

pub trait NodeRole {
    /// Sends a query to the shard group
    fn send_query(&mut self, query: &str) -> Option<String>;
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
}

impl NodeInstance {
    fn new(instance: Box<dyn NodeRole>, ip: String, port: String) -> Self {
        NodeInstance {
            instance: Some(instance),
            ip,
            port,
        }
    }

    fn change_role(&mut self, new_role: NodeType) {
        let current_instance: &mut Box<dyn NodeRole> = self.instance.as_mut().unwrap();

        match new_role {
            NodeType::Router => {
                // TODO-A: Implement data migration to another shard
                let router = Router::new(&self.ip, &self.port, None);
                *current_instance = Box::new(router);
            }
            NodeType::Shard => {
                let shard = Shard::new(&self.ip, &self.port);
                *current_instance = Box::new(shard);
            }
            _ => {
                panic!("NodeRole can only be changed to Router or Shard");
            }
        }
    }
}

pub static mut NODE_INSTANCE: Option<NodeInstance> = None;

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
    port: *const i8,
    config_file_path: *const i8,
) {
    let mut node_port = "";
    unsafe {
        if port.is_null() {
            panic!("Received a null pointer for port");
        }

        let port_str = CStr::from_ptr(port);
        node_port = match port_str.to_str() {
            Ok(str) => str,
            Err(_) => {
                panic!("Received an invalid UTF-8 string");
            }
        };
    }
    let config_path = parse_config_file(config_file_path);
    let ip = "127.0.0.1";
    new_node_instance(node_type, ip, node_port, config_path);

    let config_path_clone = config_path.clone();

    thread::spawn( move || {
        println!("Inside thread spawn");
        System::new().block_on(async {
            println!("Inside block on");
            new_raft_instance(ip, node_port, config_path_clone).await;
            println!("Returns from NEW RAFT INSTANCE 1");
        });
        println!("Returns from NEW RAFT INSTANCE 2");
    });

    // new_raft_instance(ip, node_port, config_path);
    println!("Returns from EVERYTHING");
}

fn new_node_instance(node_type: NodeType, ip: &str, port: &str, config_file_path: Option<&str>) {
    let ip_clone = ip.to_string();
    let port_clone = port.to_string();
    
    let config_file_path_clone = match config_file_path {
        Some(path) => Some(path.to_string()),
        None => None,
    };

    // Initialize node based on node type
    match node_type {
        NodeType::Router => init_router(&ip_clone, &port_clone, config_file_path_clone.as_deref()),
        NodeType::Shard => init_shard(&ip_clone, &port_clone),
        NodeType::Client => init_client(&ip_clone, &port_clone, config_file_path_clone.as_deref()),
    }
}

async fn new_raft_instance(ip: &str, original_port: &str, config_file_path: Option<&str>) {
    let nodes = get_nodes_config_raft(config_file_path.as_deref());

    // Iterate over nodes and find the name of the one that matches ip and port:
    let node_id = nodes
        .nodes
        .iter()
        .find(|node| node.ip == ip && node.port == original_port)
        .expect("Could not find node in config file").name.clone();

    let mut port: usize = original_port.parse().expect("Received an invalid port number");
    port += 2000;

    println!("Starting Raft module");
    // Directly create the RaftModule instance without creating a new runtime
    let mut raft_module = RaftModule::new(node_id.clone(), ip.to_string(), port.clone());

    // Await the start of the Raft module
    raft_module
        .start(nodes, Some(format!("../../../sharding/init_history/init_{}", node_id)))
        .await;

    println!("Raft module started");

    let ctx = raft_module.address.expect("Raft module address is None");

    println!("PRE-FIN RAFT INSTANCE");
    RaftModule::listen_for_connections(port, ip.to_string(), node_id, ctx);
    println!("FIN RAFT INSTANCE");
}

fn init_router(ip: &str, port: &str, config_file_path: Option<&str>) {
    let router = Router::new(ip, port, config_file_path);

    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(router.clone()),
            ip.to_string(),
            port.to_string(),
        ));
    }

    let shared_router: Arc<Mutex<Router>> = Arc::new(Mutex::new(router));
    let ip_clone = ip.to_string();
    let port_clone = port.to_string();
    let handle = thread::spawn(move || {
        Router::wait_for_client(shared_router, ip_clone, port_clone);
    });

    println!("Router node initializes");
    handle.join().unwrap();
}

fn init_shard(ip: &str, port: &str) {
    println!("Sharding node initializing");
    let shard = Shard::new(ip, port);

    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(shard.clone()),
            ip.to_string(),
            port.to_string(),
        ));
    }

    let shared_shard = Arc::new(Mutex::new(shard));
    let ip_clone = ip.to_string();
    let port_clone = port.to_string();
    let _handle = thread::spawn(move || {
        Shard::accept_connections(shared_shard, ip_clone, port_clone);
    });

    println!("Sharding node initializes");
}

fn init_client(ip: &str, port: &str, config_file_path: Option<&str>) {
    println!("Client node initializing");
    unsafe {
        NODE_INSTANCE = Some(NodeInstance::new(
            Box::new(Client::new(ip, port, config_file_path)),
            ip.to_string(),
            port.to_string(),
        ));
    }
    println!("Client node initializes");
}

fn parse_config_file(config_file_path: *const i8) -> Option<&'static str> {
    match config_file_path.is_null() {
        true => None,
        false => unsafe {
            let config_path_str = CStr::from_ptr(config_file_path);
            let config_path = match config_path_str.to_str() {
                Ok(str) => str,
                Err(_) => {
                    panic!("Received an invalid UTF-8 string for config path");
                }
            };
            Some(config_path)
        },
    }
}
