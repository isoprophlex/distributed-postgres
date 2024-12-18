use std::io;
use sysinfo::Disks;

/// This struct represents the Memory Manager in the distributed system.
/// It will manage the memory of the node and will be used to determine if the node should accept new requests.
/// It holds the unavailable_memory_perc, which is the percentage of memory that should not be used and is reserved for the system to use for other purposes.
pub struct MemoryManager {
    unavailable_memory_perc: f64,
    pub available_memory_perc: f64,
}

impl MemoryManager {
    /// Creates a new MemoryManager.
    pub fn new(unavailable_memory_perc: f64) -> Self {
        let available_memory_perc =
            match Self::get_available_memory_percentage(unavailable_memory_perc) {
                Some(perc) => perc,
                None => panic!("[MemoryManager] Failed to get available memory"),
            };
        MemoryManager {
            unavailable_memory_perc,
            available_memory_perc,
        }
    }

    /// Updates the available memory percentage.
    pub fn update(&mut self) -> Result<(), io::Error> {
        self.available_memory_perc =
            match Self::get_available_memory_percentage(self.unavailable_memory_perc) {
                Some(perc) => perc,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "[MemoryManager] Failed to get available memory",
                    ))
                }
            };
        Ok(())
    }

    /// Returns the available memory percentage.
    fn get_available_memory_percentage(unavailable_memory_perc: f64) -> Option<f64> {
        if unavailable_memory_perc == 100.0 {
            return Some(0.0);
        }

        // Create a Disk object
        let disks = Disks::new_with_refreshed_list();

        let mut total_space: u64 = 0;
        let mut available_space: u64 = 0;

        // Get the total and available space of the disk
        for disk in &disks {
            total_space += disk.total_space();
            available_space += disk.available_space();
        }

        // if percentage is greater than 1, it means that the total of space used exceeds the threshold.
        // If so, return 0
        let total = total_space as f64;
        let threshold_size = total * (unavailable_memory_perc / 100.0);

        if threshold_size > available_space as f64 {
            return Some(0.0);
        }

        let usable_available_space = available_space as f64 - threshold_size;
        let usable_total_space = total - threshold_size / 100.0;

        let percentage = usable_available_space / usable_total_space * 100.0;
        if percentage > 100.0 {
            return Some(0.0);
        }
        Some(percentage)
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_get_available_memory_percentage() {
        let unavailable_memory_perc = 0.0;
        let available_memory_perc =
            MemoryManager::get_available_memory_percentage(unavailable_memory_perc);
        assert!(available_memory_perc.is_some());
    }

    #[test]
    fn test_get_available_memory_percentage_threashold_is_total() {
        let unavailable_memory_perc = 100.0;
        let available_memory_perc =
            MemoryManager::get_available_memory_percentage(unavailable_memory_perc).unwrap();
        assert_eq!(available_memory_perc, 0.0);
    }

    #[test]
    fn test_get_available_memory_percentage_threashold_exceeds_available_space() {
        let unavailable_memory_perc = 90.0;
        let available_memory_perc =
            MemoryManager::get_available_memory_percentage(unavailable_memory_perc).unwrap();
        assert_eq!(available_memory_perc, 0.0);
    }
}
