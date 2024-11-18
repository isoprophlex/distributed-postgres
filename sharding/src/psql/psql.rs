use std::ffi::CStr;
extern crate users;
use super::super::node::node::*;

#[no_mangle]
pub extern "C" fn SendQueryToShard(query_data: *const i8) -> bool {
    unsafe {
        if query_data.is_null() {
            eprintln!("Received a null pointer");
            return false;
        }

        let c_str = CStr::from_ptr(query_data);
        let query = match c_str.to_str() {
            Ok(str) => str,
            Err(_) => {
                eprintln!("Received an invalid UTF-8 string");
                return false;
            }
        };

        handle_query(query.trim())
    }
}

fn handle_query(query: &str) -> bool {
    let node_instance = get_node_role();
    match node_instance.send_query(query) {
        Some(_) => true,
        None => false,
    }
}
