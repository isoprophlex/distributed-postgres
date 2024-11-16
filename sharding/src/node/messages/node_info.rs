use std::str::FromStr;

use crate::utils::node_config::get_nodes_config;

#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub ip: String,
    pub port: String,
    pub name: String,
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

        let name = match parts.next() {
            Some(name) => name.to_string(),
            None => return Err("Missing name"),
        };

        Ok(NodeInfo { ip, port, name })
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

pub fn find_name_for_node(ip: String, port: String) -> Option<String> {
    let config = get_nodes_config();
    for node in config.nodes {
        if node.ip == ip && node.port == port {
            return Some(node.name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_info_from_str() {
        let node_info = NodeInfo::from_str("127.0.0.0:5344:node1").unwrap();
        assert_eq!(node_info.ip, "127.0.0.0");
        assert_eq!(node_info.port, "5344");
        assert_eq!(node_info.name, "node1");
    }
}
