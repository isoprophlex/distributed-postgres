use serde::Deserialize;
use std::fs;

const CONFIG_FILE_PATH: &str = "../../../sharding/src/node/config/nodes_config.yaml";
const MEMORY_CONFIG_FILE_PATH: &str = "../../../sharding/src/node/config/memory_config.yaml";
pub const INIT_HISTORY_FILE_PATH: &str = "../../../sharding/init_history/init_";

#[derive(Debug, Deserialize)]
pub struct NodesConfig {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    pub ip: String,
    pub port: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct LocalNode {
    pub unavailable_memory_perc: f64,
}

pub fn get_config_path() -> String {
    CONFIG_FILE_PATH.to_string()
}

pub fn get_nodes_config() -> NodesConfig {
    // read from 'config.yaml' to get the NodeConfig
    let config_file_path = get_config_path();

    let config_content = match fs::read_to_string(config_file_path) {
        Ok(content) => content,
        Err(_) => {
            return NodesConfig { nodes: vec![] };
        }
    };

    serde_yaml::from_str(&config_content).expect("Should have been able to parse the YAML")
}

pub fn get_nodes_config_raft() -> raft::node_config::NodesConfig {
    // read from 'config.yaml' to get the NodeConfig from raft
    let config_file_path = get_config_path();

    let config_content =
        fs::read_to_string(config_file_path).expect("Should have been able to read the file");

    serde_yaml::from_str(&config_content).expect("Should have been able to parse the YAML")
}

pub fn get_memory_config() -> LocalNode {
    let config_content = match fs::read_to_string(MEMORY_CONFIG_FILE_PATH) {
        Ok(content) => content,
        Err(_) => {
            return LocalNode {
                unavailable_memory_perc: 0.0,
            };
        }
    };

    serde_yaml::from_str(&config_content).expect("Should have been able to parse the YAML")
}
