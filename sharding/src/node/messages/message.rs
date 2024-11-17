use super::{message_data::MessageData, node_info::NodeInfo};
use crate::{
    node::tables_id_info::TablesIdInfo,
    utils::common::{ConvertToString, FromString},
};
use std::fmt;

/// MessageType enum shows which command is being sent
#[derive(Debug, PartialEq, Clone)]
pub enum MessageType {
    InitConnection,
    AskMemoryUpdate,
    MemoryUpdate,
    Agreed,
    Denied,
    GetRouter,
    RouterId,
    HelloFromNode,
    NoRouterData,
    Query,
    QueryResponse,
}

#[derive(Clone)]
/// Message struct is used to send commands between clients and server
pub struct Message {
    message_type: MessageType,
    payload: Option<f64>,
    max_ids: Option<TablesIdInfo>,
    node_info: Option<NodeInfo>,
    query_data: Option<String>,
}

/// Implementing Display for Message
impl fmt::Debug for Message {
    
    
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Message")
            .field("message_type", &self.message_type)
            .field("payload", &self.payload)
            .field("max_ids", &self.max_ids)
            .field("node_info", &self.node_info)
            .field("query_data", &self.query_data)
            .finish()
    }
}

/// Message Creation
impl Message {
    // --- Constructors ---

    pub fn new_init_connection(node_info: NodeInfo) -> Self {
        Message {
            message_type: MessageType::InitConnection,
            payload: None,
            max_ids: None,
            node_info: Some(node_info),
            query_data: None,
        }
    }

    pub fn new_ask_memory_update() -> Self {
        Message {
            message_type: MessageType::AskMemoryUpdate,
            payload: None,
            max_ids: None,
            node_info: None,
            query_data: None,
        }
    }

    pub fn new_memory_update(payload: f64, max_ids: TablesIdInfo) -> Self {
        Message {
            message_type: MessageType::MemoryUpdate,
            payload: Some(payload),
            node_info: None,
            query_data: None,
            max_ids: Some(max_ids),
        }
    }

    pub fn new_agreed(memory_percentage: f64, max_ids: TablesIdInfo) -> Self {
        Message {
            message_type: MessageType::Agreed,
            payload: Some(memory_percentage),
            max_ids: Some(max_ids),
            node_info: None,
            query_data: None,
        }
    }

    pub fn new_denied() -> Self {
        Message {
            message_type: MessageType::Denied,
            payload: None,
            max_ids: None,
            node_info: None,
            query_data: None,
        }
    }

    pub fn new_get_router() -> Self {
        Message {
            message_type: MessageType::GetRouter,
            payload: None,
            max_ids: None,
            node_info: None,
            query_data: None,
        }
    }

    pub fn new_router_id(node_info: NodeInfo) -> Self {
        Message {
            message_type: MessageType::RouterId,
            payload: None,
            max_ids: None,
            node_info: Some(node_info),
            query_data: None,
        }
    }

    pub fn new_hello_from_node(node_info: NodeInfo) -> Self {
        Message {
            message_type: MessageType::HelloFromNode,
            payload: None,
            max_ids: None,
            node_info: Some(node_info),
            query_data: None,
        }
    }

    pub fn new_no_router_data() -> Self {
        Message {
            message_type: MessageType::NoRouterData,
            payload: None,
            max_ids: None,
            node_info: None,
            query_data: None,
        }
    }

    pub fn new_query(sender_info: Option<NodeInfo>, query: String) -> Self {
        Message {
            message_type: MessageType::Query,
            payload: None,
            max_ids: None,
            node_info: sender_info,
            query_data: Some(query),
        }
    }

    pub fn new_query_response(query_response: String) -> Self {
        Message {
            message_type: MessageType::QueryResponse,
            payload: None,
            max_ids: None,
            node_info: None,
            query_data: Some(query_response),
        }
    }

    // --- Method to get the data ---

    pub fn get_data(&self) -> MessageData {
        match self.message_type {
            MessageType::InitConnection | MessageType::RouterId | MessageType::HelloFromNode => {
                if let Some(ref node_info) = self.node_info {
                    MessageData::new_node_info(node_info.clone())
                } else {
                    MessageData::new_none()
                }
            }
            MessageType::MemoryUpdate | MessageType::Agreed => {
                let max_ids = match self.max_ids.clone() {
                    Some(max_ids) => max_ids,
                    None => TablesIdInfo::new(),
                };
                if let Some(payload) = self.payload {
                    println!(
                        "Payload: {}, Max Ids: {}",
                        payload,
                        max_ids.convert_to_string()
                    );
                    MessageData::new_payload(payload, max_ids)
                } else {
                    MessageData::new_none()
                }
            }
            MessageType::Query => {
                if let Some(ref query) = self.query_data {
                    if let Some(ref node_info) = self.node_info {
                        return MessageData::new_query(query.clone(), Some(node_info.clone()));
                    }
                }
                MessageData::new_none()
            }
            MessageType::QueryResponse => {
                if let Some(ref query_response) = self.query_data {
                    return MessageData::new_query_response(query_response.clone());
                }
                MessageData::new_none()
            }
            _ => MessageData::new_none(),
        }
    }

    pub fn get_message_type(&self) -> MessageType {
        self.message_type.clone()
    }
}

impl Message {

    // Serialize the Message to a String
    pub fn to_string(&self) -> String {
        let mut result = String::new();

        // Message Type
        result.push_str(match self.message_type {
            MessageType::InitConnection => "INIT_CONNECTION",
            MessageType::AskMemoryUpdate => "ASK_MEMORY_UPDATE",
            MessageType::MemoryUpdate => "MEMORY_UPDATE",
            MessageType::Agreed => "AGREED",
            MessageType::Denied => "DENIED",
            MessageType::GetRouter => "GET_ROUTER",
            MessageType::RouterId => "ROUTER_ID",
            MessageType::HelloFromNode => "HELLO_FROM_NODE",
            MessageType::NoRouterData => "NO_ROUTER_DATA",
            MessageType::Query => "QUERY",
            MessageType::QueryResponse => "QUERY_RESPONSE",
        });

        // Payload
        result.push(' ');
        if let Some(payload) = self.payload {
            result.push_str(&payload.to_string());
        } else {
            result.push_str("None");
        }

        // Max Ids
        result.push(' ');
        let max_ids = match self.max_ids.clone() {
            Some(max_ids) => max_ids,
            None => TablesIdInfo::new(),
        };
        if !max_ids.is_empty() {
            result.push_str(&max_ids.clone().convert_to_string());
        } else {
            result.push_str("None");
        }

        // Node Info
        result.push(' ');
        if let Some(node_info) = &self.node_info {
            result.push_str(&node_info.ip);
            result.push(':');
            result.push_str(&node_info.port);
            result.push(':');
            result.push_str(&node_info.name);
        } else {
            result.push_str("None");
        }

        // Query Data
        result.push(' ');
        if let Some(query) = &self.query_data {
            result.push_str(&query);
        } else {
            result.push_str("None");
        }
        result + "\n"
    }

    // Deserialize the Message from a String
    pub fn from_string(input: &str) -> Result<Message, &'static str> {
        let mut parts = input.split_whitespace();

        // Message Type
        let message_type = match parts.next() {
            Some("INIT_CONNECTION") => MessageType::InitConnection,
            Some("ASK_MEMORY_UPDATE") => MessageType::AskMemoryUpdate,
            Some("MEMORY_UPDATE") => MessageType::MemoryUpdate,
            Some("AGREED") => MessageType::Agreed,
            Some("DENIED") => MessageType::Denied,
            Some("GET_ROUTER") => MessageType::GetRouter,
            Some("ROUTER_ID") => MessageType::RouterId,
            Some("HELLO_FROM_NODE") => MessageType::HelloFromNode,
            Some("NO_ROUTER_DATA") => MessageType::NoRouterData,
            Some("QUERY") => MessageType::Query,
            Some("QUERY_RESPONSE") => MessageType::QueryResponse,
            _ => return Err("Invalid message type"),
        };

        // Payload
        let payload = match parts.next() {
            Some("None") => None,
            Some(payload) => match payload.parse() {
                Ok(payload) => Some(payload),
                Err(_) => return Err("Invalid payload"),
            }
            None => None,
        };

        // Max Ids
        let max_ids = match parts.next() {
            Some("None") => None,
            Some(max_ids) => Some(TablesIdInfo::from_string(max_ids)),
            None => None,
        };

        // Node Info
        let node_info = match parts.next() {
            Some("None") => None,
            Some(node_info) => Some(node_info.parse()?),
            None => None,
        };

        // Query Data
        let query = match parts.next() {
            Some("None") => None,
            Some(query) => {
                let mut query = query.to_string();
                query.push(' ');
                for part in parts.clone() {
                    query.push_str(part);
                    query.push(' ');
                }
                match query.split(';').next() {
                    Some(query) => Some(query.to_string()),
                    None => None,
                }
            }
            None => None,
        };

        Ok(Message {
            message_type,
            payload,
            max_ids,
            node_info,
            query_data: query,
        })
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Message) -> bool {
        self.message_type == other.message_type && self.payload == other.payload
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    // test initializers

    #[test]
    fn test_new_init_connection() {
        let node_info = NodeInfo {
            ip: "1".to_string(),
            port: "2".to_string(),
            name: "node_name".to_string(),
        };
        let message = Message::new_init_connection(node_info.clone());
        assert_eq!(
            message,
            Message {
                message_type: MessageType::InitConnection,
                payload: None,
                max_ids: None,
                node_info: Some(node_info),
                query_data: None,
            }
        );
    }

    #[test]
    fn test_new_ask_memory_update() {
        let message = Message::new_ask_memory_update();
        assert_eq!(
            message,
            Message {
                message_type: MessageType::AskMemoryUpdate,
                payload: None,
                max_ids: None,
                node_info: None,
                query_data: None,
            }
        );
    }

    #[test]
    fn test_new_memory_update() {
        let max_ids = TablesIdInfo::from_string("employees:3,departments:5");
        let message = Message::new_memory_update(0.5, max_ids.clone());
        assert_eq!(
            message,
            Message {
                message_type: MessageType::MemoryUpdate,
                payload: Some(0.5),
                max_ids: Some(max_ids),
                node_info: None,
                query_data: None,
            }
        );
    }

    #[test]
    fn test_new_agreed() {
        let max_ids = TablesIdInfo::from_string("employees:3,departments:5");
        let message = Message::new_agreed(0.5, max_ids.clone());
        assert_eq!(
            message,
            Message {
                message_type: MessageType::Agreed,
                payload: Some(0.5),
                max_ids: Some(max_ids),
                node_info: None,
                query_data: None,
            }
        );
    }

    #[test]
    fn test_new_denied() {
        let message = Message::new_denied();
        assert_eq!(
            message,
            Message {
                message_type: MessageType::Denied,
                payload: None,
                max_ids: None,
                node_info: None,
                query_data: None,
            }
        );
    }

    #[test]
    fn test_new_get_router() {
        let message = Message::new_get_router();
        assert_eq!(
            message,
            Message {
                message_type: MessageType::GetRouter,
                payload: None,
                max_ids: None,
                node_info: None,
                query_data: None,
            }
        );
    }

    #[test]
    fn test_new_router_id() {
        let node_info = NodeInfo {
            ip: "1".to_string(),
            port: "2".to_string(),
            name: "node_name".to_string(),
        };
        let message = Message::new_router_id(node_info.clone());
        assert_eq!(
            message,
            Message {
                message_type: MessageType::RouterId,
                payload: None,
                max_ids: None,
                node_info: Some(node_info),
                query_data: None,
            }
        );
    }

    #[test]
    fn test_new_no_router_data() {
        let message = Message::new_no_router_data();
        assert_eq!(
            message,
            Message {
                message_type: MessageType::NoRouterData,
                payload: None,
                max_ids: None,
                node_info: None,
                query_data: None,
            }
        );
    }

    #[test]
    fn test_new_query() {
        let message = Message::new_query(
            Some(NodeInfo {
                ip: "1".to_string(),
                port: "2".to_string(),
                name: "node_name".to_string(),
            }),
            "SELECT * FROM table".to_string(),
        );
        assert_eq!(
            message,
            Message {
                message_type: MessageType::Query,
                payload: None,
                max_ids: None,
                node_info: None,
                query_data: Some("SELECT * FROM table".to_string()),
            }
        );
    }

    #[test]
    fn test_get_data_payload() {
        let max_ids = TablesIdInfo::from_string("employees:3,departments:5");
        let message = Message {
            message_type: MessageType::MemoryUpdate,
            payload: Some(0.5),
            max_ids: Some(max_ids.clone()),
            node_info: None,
            query_data: None,
        };
        assert_eq!(message.get_data(), MessageData::new_payload(0.5, max_ids));
    }

    #[test]
    fn test_get_data_node_info() {
        let node_info = NodeInfo {
            ip: "1".to_string(),
            port: "2".to_string(),
            name: "node_name".to_string(),
        };

        let message = Message {
            message_type: MessageType::InitConnection,
            payload: None,
            max_ids: None,
            node_info: Some(node_info.clone()),
            query_data: None,
        };
        assert_eq!(message.get_data(), MessageData::new_node_info(node_info));
    }

    #[test]
    fn test_get_data_query() {
        let message = Message {
            message_type: MessageType::Query,
            payload: None,
            max_ids: None,
            node_info: Some(NodeInfo {
                ip: "1".to_string(),
                port: "2".to_string(),
                name: "node_name".to_string(),
            }),
            query_data: Some("SELECT * FROM table".to_string()),
        };
        assert_eq!(
            message.get_data(),
            MessageData::new_query(
                "SELECT * FROM table".to_string(),
                Some(NodeInfo {
                    ip: "1".to_string(),
                    port: "2".to_string(),
                    name: "node_name".to_string(),
                })
            )
        );
    }

    #[test]
    fn test_get_data_none() {
        let message = Message {
            message_type: MessageType::InitConnection,
            payload: None,
            max_ids: None,
            node_info: None,
            query_data: None,
        };
        assert_eq!(message.get_data(), MessageData::new_none());
    }

    #[test]
    fn test_message_to_string() {
        let max_ids = TablesIdInfo::from_string("employees:3,departments:5");
        let message = Message {
            message_type: MessageType::InitConnection,
            payload: Some(0.5),
            max_ids: Some(max_ids.clone()),
            node_info: None,
            query_data: None,
        };
        println!("-{}-", message.to_string());
        let options = [
            "INIT_CONNECTION 0.5 employees:3,departments:5 None None\n",
            "INIT_CONNECTION 0.5 departments:5,employees:3 None None\n",
        ];

        assert!(options.contains(&&message.to_string().as_str()));
    }

    #[test]
    fn test_message_from_string() {
        let max_ids = TablesIdInfo::from_string("employees:3,departments:5");
        let message = Message {
            message_type: MessageType::InitConnection,
            payload: Some(0.5),
            max_ids: Some(max_ids.clone()),
            node_info: None,
            query_data: None,
        };
        let message_string = message.to_string();
        assert_eq!(Message::from_string(&message_string).unwrap(), message);
    }

    #[test]
    fn test_message_from_string_with_node_info() {
        let node_info = NodeInfo {
            ip: "1".to_string(),
            port: "2".to_string(),
            name: "node_name".to_string(),
        };

        let max_ids = TablesIdInfo::from_string("employees:3,departments:5");
        let message = Message {
            message_type: MessageType::InitConnection,
            payload: Some(0.5),
            max_ids: Some(max_ids.clone()),
            node_info: Some(node_info.clone()),
            query_data: None,
        };
        let message_string = message.to_string();
        assert_eq!(Message::from_string(&message_string).unwrap(), message);
    }

    #[test]
    fn test_message_from_string_with_query() {
        let max_ids = TablesIdInfo::from_string("employees:3,departments:5");
        let message = Message {
            message_type: MessageType::Query,
            payload: Some(0.5),
            max_ids: Some(max_ids.clone()),
            node_info: Some(NodeInfo {
                ip: "1".to_string(),
                port: "2".to_string(),
                name: "node_name".to_string(),
            }),
            query_data: Some("SELECT * FROM table".to_string()),
        };
        let message_string = message.to_string();
        assert_eq!(Message::from_string(&message_string).unwrap(), message);
    }
}
