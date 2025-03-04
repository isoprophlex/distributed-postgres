use indexmap::IndexMap;
use inline_colorization::*;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    sync::{Arc, Mutex},
};

use super::tables_id_info::TablesIdInfo;

#[derive(Debug, Clone)]
pub(crate) struct ShardManager {
    shards: Arc<Mutex<BinaryHeap<ShardManagerObject>>>,
    shard_max_ids: Arc<Mutex<IndexMap<String, TablesIdInfo>>>,
}

impl ShardManager {
    pub fn new() -> Self {
        ShardManager {
            shards: Arc::new(Mutex::new(BinaryHeap::new())),
            shard_max_ids: Arc::new(Mutex::new(IndexMap::new())),
        }
    }

    pub fn add_shard(&mut self, value: f64, shard_id: String) {
        let object = ShardManagerObject {
            key: value,
            value: shard_id,
        };
        println!("Adding shard: {:?}", object);
        let mut shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                println!("{color_bright_red}Failed to lock shards{style_reset}");
                return;
            }
        };
        shards.push(object);
    }

    pub fn peek(&self) -> Option<String> {
        println!("Peeking shards: {:?}", self.shards);
        let shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                println!("{color_bright_red}Failed to lock shards{style_reset}");
                return None;
            }
        };

        match shards.peek() {
            Some(object) => Some(object.value.clone()),
            None => None,
        }
    }

    pub fn count(&self) -> usize {
        let shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                println!("{color_bright_red}Failed to lock shards{style_reset}");
                return 0;
            }
        };

        shards.len()
    }

    /// Updates the memory of a shard and reorders the shards based on the new memory.
    /// If the memory is higher than the current top shard, it will become the new top shard.
    /// If the memory is lower than the current top shard, it will be placed in the correct position in the heap.
    /// If the memory is zero, the shard will be at the base of the heap until it is updated once again.
    pub fn update_shard_memory(&mut self, memory: f64, shard_id: String) {
        println!(
            "{color_bright_green}Updating shard memory: {} to {}{style_reset}",
            shard_id, memory
        );

        self.delete(shard_id.clone());
        self.add_shard(memory, shard_id);

        println!(
            "{color_bright_green}Shard memory updated: {:?}{style_reset}",
            self.shards
        );
    }

    fn pop(&mut self) -> Option<String> {
        let mut shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                println!("{color_bright_red}Failed to lock shards{style_reset}");
                return None;
            }
        };

        match shards.pop() {
            Some(object) => Some(object.value),
            None => None,
        }
    }

    // This is not the most efficient way. If you'd like to improve it, we have thought about options: using a different data structure, or if the query affects all shards, clear the heap and add them from scratch. This needs to be thinked through, because the router handles each of the shards separately.
    // Anyway, we are out of time, so this is the best we can do for now.
    /// Deletes a shard from the heap.
    pub fn delete(&mut self, shard_id: String) {
        let peeked_shard_id = match self.peek() {
            Some(shard_id) => shard_id,
            None => {
                return;
            }
        };

        if shard_id == peeked_shard_id {
            self.pop();
            return;
        }

        let mut shards = match self.shards.lock() {
            Ok(shards) => shards,
            Err(_) => {
                println!("{color_bright_red}Failed to lock shards{style_reset}");
                return;
            }
        };

        let mut new_shards = BinaryHeap::new();

        while let Some(shard) = shards.pop() {
            if shard.value == shard_id {
                continue;
            } else {
                new_shards.push(shard);
            }
        }
        *shards = new_shards;
    }

    pub fn save_max_ids_for_shard(&mut self, shard_id: String, tables_id_info: TablesIdInfo) {
        let shard_max_ids = match self.shard_max_ids.lock() {
            Ok(shard_max_ids) => shard_max_ids,
            Err(_) => {
                println!("{color_bright_red}Failed to lock shard_max_ids{style_reset}");
                return;
            }
        };
        let mut shard_max_ids = shard_max_ids;
        shard_max_ids.insert(shard_id, tables_id_info);
    }

    pub fn get_table_names_for_all(&self) -> Vec<String> {
        let shard_max_ids = match self.shard_max_ids.lock() {
            Ok(shard_max_ids) => shard_max_ids,
            Err(_) => {
                println!("{color_bright_red}Failed to lock shard_max_ids{style_reset}");
                return Vec::new();
            }
        };

        let mut table_names = Vec::new();
        for tables_id_info in shard_max_ids.values() {
            for table_name in tables_id_info.keys() {
                table_names.push(table_name.clone());
            }
        }

        table_names
    }

    pub fn get_max_ids_for_shard_table(&self, shard_id: &str, table: &str) -> Option<i64> {
        let shard_max_ids = match self.shard_max_ids.lock() {
            Ok(shard_max_ids) => shard_max_ids,
            Err(_) => {
                println!("{color_bright_red}Failed to lock shard_max_ids{style_reset}");
                return None;
            }
        };

        println!("shard_max_ids: {:?}", shard_max_ids);
        println!("shard_id: {:?}, table: {:?}", shard_id, table);
        match shard_max_ids.get(shard_id) {
            Some(tables_id_info) => match tables_id_info.get(table) {
                Some(max_id) => Some(*max_id),
                None => None,
            },
            None => None,
        }
    }
}

#[derive(Debug, Clone)]
struct ShardManagerObject {
    key: f64,
    value: String,
}

impl Ord for ShardManagerObject {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.partial_cmp(other) {
            Some(ordering) => ordering,
            None => Ordering::Equal,
        }
    }
}

impl PartialOrd for ShardManagerObject {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key.partial_cmp(&other.key)
    }
}

impl PartialEq for ShardManagerObject {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for ShardManagerObject {}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn test_shard_manager_init_empty() {
        let shard_manager = ShardManager::new();
        assert_eq!(shard_manager.peek(), None);
    }

    #[test]
    fn test_shard_manager_add_shard() {
        let mut shard_manager = ShardManager::new();
        shard_manager.add_shard(1.0, "shard1".to_string());
        assert_eq!(shard_manager.peek(), Some("shard1".to_string()));
    }

    #[test]
    fn test_shard_manager_add_multiple_shards_returns_max_shard() {
        let mut shard_manager = ShardManager::new();
        shard_manager.add_shard(1.0, "shard1".to_string());
        shard_manager.add_shard(2.0, "shard2".to_string());
        shard_manager.add_shard(3.0, "shard3".to_string());
        shard_manager.add_shard(4.0, "shard4".to_string());
        shard_manager.add_shard(5.0, "shard5".to_string());

        assert_eq!(shard_manager.peek(), Some("shard5".to_string()));
    }

    #[test]
    fn test_shard_manager_update_shard_memory_new_top() {
        let mut shard_manager = ShardManager::new();
        shard_manager.add_shard(1.0, "shard1".to_string());
        shard_manager.add_shard(2.0, "shard2".to_string());
        shard_manager.add_shard(3.0, "shard3".to_string());
        shard_manager.add_shard(4.0, "shard4".to_string());
        shard_manager.add_shard(5.0, "shard5".to_string());

        shard_manager.update_shard_memory(10.0, "shard3".to_string());

        assert_eq!(shard_manager.peek(), Some("shard3".to_string()));
    }

    #[test]
    fn test_shard_manager_update_shard_memory_from_top() {
        let mut shard_manager = ShardManager::new();
        shard_manager.add_shard(1.0, "shard1".to_string());
        shard_manager.add_shard(2.0, "shard2".to_string());
        shard_manager.add_shard(3.0, "shard3".to_string());
        shard_manager.add_shard(4.0, "shard4".to_string());
        shard_manager.add_shard(5.0, "shard5".to_string());

        shard_manager.update_shard_memory(0.0, "shard5".to_string());

        assert_eq!(shard_manager.peek(), Some("shard4".to_string()));
    }

    #[test]
    fn test_shard_manager_update_shard_memory_from_top_and_pop() {
        let mut shard_manager = ShardManager::new();
        shard_manager.add_shard(1.0, "shard1".to_string());
        shard_manager.add_shard(2.0, "shard2".to_string());
        shard_manager.add_shard(3.0, "shard3".to_string());
        shard_manager.add_shard(4.0, "shard4".to_string());
        shard_manager.add_shard(5.0, "shard5".to_string());

        shard_manager.update_shard_memory(0.0, "shard5".to_string());
        shard_manager.pop();

        assert_eq!(shard_manager.peek(), Some("shard3".to_string()));
    }

    #[test]
    fn test_shard_manager_pop() {
        let mut shard_manager = ShardManager::new();
        shard_manager.add_shard(1.0, "shard1".to_string());
        shard_manager.add_shard(2.0, "shard2".to_string());
        shard_manager.add_shard(3.0, "shard3".to_string());
        shard_manager.add_shard(4.0, "shard4".to_string());
        shard_manager.add_shard(5.0, "shard5".to_string());

        shard_manager.pop();
        assert_eq!(shard_manager.peek(), Some("shard4".to_string()));
    }
}
