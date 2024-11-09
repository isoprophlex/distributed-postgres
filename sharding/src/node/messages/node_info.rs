use std::str::FromStr;

use crate::utils::node_config::get_nodes_config;

#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub ip: String,
    pub port: String,
}

impl FromStr for NodeInfo {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<NodeInfo, &'static str> {
        // split the input string by ':'
        let mut parts = input.split(':');

        let ip = match parts.next() {
            Some(ip) => ip.to_string(),
            None => return Err("Missing ip"),
        };

        let port = match parts.next() {
            Some(port) => port.to_string(),
            None => return Err("Missing port"),
        };

        Ok(NodeInfo { ip, port })
    }
}

impl PartialEq for NodeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip && self.port == other.port
    }
}

impl std::fmt::Display for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

pub fn find_name_for_node(node_info: &NodeInfo) -> Option<String> {
    let config = get_nodes_config();
    for node in config.nodes {
        let candidate_info = NodeInfo {
            ip: node.ip,
            port: node.port,
        };
        if candidate_info == *node_info {
            return Some(node.name);
        }
    }
    None
}