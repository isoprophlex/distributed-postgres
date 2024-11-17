// tests/integration_test.rs

use core::panic;
use postgres::{Client, NoTls};
use sharding::node::node::{get_node_role, InitNodeInstance, NodeRole, NodeType};
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    thread,
};
use users::get_current_username;

#[cfg(test)]
mod integration_test {
    use std::sync::{Arc, Mutex};

    use sharding::{
        node::{router::Router, shard::Shard},
        utils::node_config::Node,
    };

    use super::*;

    fn setup_connection(host: &str, port: &str, db_name: &str) -> Option<Client> {
        let username = match get_current_username() {
            Some(username) => username.to_string_lossy().to_string(),
            None => self::panic!("Failed to get current username"),
        };

        match Client::connect(
            format!(
                "host={} port={} user={} dbname={}",
                host, port, username, db_name
            )
            .as_str(),
            NoTls,
        ) {
            Ok(client) => Some(client),
            Err(e) => {
                self::panic!("Failed to connect to the database: {:?}", e);
            }
        }
    }

    struct TestContext;

    impl Drop for TestContext {
        fn drop(&mut self) {
            delete_test_clusters();
        }
    }

    #[test]
    fn test_nodes_initialize_empty() {
        let _context = TestContext;

        create_and_init_cluster(b"test-shard\n", "s", "localhost", "5433");
        create_and_init_cluster(b"test-router\n", "r", "localhost", "5434");

        let mut router_connection: Client =
            setup_connection("localhost", "5433", "template1").unwrap();
        let mut shard_connection: Client =
            setup_connection("localhost", "5434", "template1").unwrap();

        // Count user tables, excluding system tables
        let row = router_connection
            .query_one(
                "SELECT COUNT(*) FROM pg_catalog.pg_tables WHERE schemaname = 'public'",
                &[],
            )
            .unwrap();
        let count: i64 = row.get(0);
        assert_eq!(count, 0);

        let row = shard_connection
            .query_one(
                "SELECT COUNT(*) FROM pg_catalog.pg_tables WHERE schemaname = 'public'",
                &[],
            )
            .unwrap();
        let count: i64 = row.get(0);
        assert_eq!(count, 0);

        stop_cluster(b"test-router\n");
        stop_cluster(b"test-shard\n");
    }

    #[test]
    fn test_create_table() {
        let _context = TestContext;
        create_and_init_cluster(b"test-shard1\n", "s", "localhost", "5433");

        let shard = Shard::new("localhost", "5433");
        let shared_shard = Arc::new(Mutex::new(shard));
        let ip_clone = "localhost".to_string();
        let port_clone = "5433".to_string();
        let _handle = thread::spawn(move || {
            Shard::accept_connections(shared_shard, ip_clone, port_clone);
            println!("Shard comes back from accept_connections");
        });

        thread::sleep(std::time::Duration::from_secs(15));
        create_and_init_cluster(b"test-router1\n", "r", "localhost", "5434");

        let mut shard_connection: Client =
            setup_connection("localhost", "5433", "template1").unwrap();

        let mut router = match Router::new("localhost", "5434") {
            Some(router) => router,
            None => {
                self::panic!("Failed to create router");
            }
        };

        let node: Node = Node {
            ip: "localhost".to_string(),
            port: "5433".to_string(),
            name: "test-shard1".to_string(),
        };
        router.configure_shard_connection_to(node);

        // Create a table on the router
        assert!(router
            .send_query("DROP TABLE IF EXISTS test_table;")
            .is_some());
        assert!(router
            .send_query("CREATE TABLE test_table (id INT PRIMARY KEY);")
            .is_some());

        // Count user tables in the shard, excluding system tables. Should be one.
        let row = shard_connection
            .query_one(
                "SELECT COUNT(*) FROM pg_catalog.pg_tables WHERE schemaname = 'public';",
                &[],
            )
            .unwrap();
        let count: i64 = row.get(0);
        assert_eq!(count, 1);

        stop_cluster(b"test-shard1\n");
        stop_cluster(b"test-router1\n");
    }

    #[test]
    fn test_insert_into_table_select_and_delete() {
        let _context = TestContext;
        create_and_init_cluster(b"test-shard1\n", "s", "localhost", "5433");

        let shard = Shard::new("localhost", "5433");
        let shared_shard = Arc::new(Mutex::new(shard));
        let ip_clone = "localhost".to_string();
        let port_clone = "5433".to_string();
        let _handle = thread::spawn(move || {
            Shard::accept_connections(shared_shard, ip_clone, port_clone);
            println!("Shard comes back from accept_connections");
        });

        thread::sleep(std::time::Duration::from_secs(15));
        create_and_init_cluster(b"test-router1\n", "r", "localhost", "5434");

        let mut shard_connection: Client =
            setup_connection("localhost", "5433", "template1").unwrap();

        let mut router = match Router::new("localhost", "5434") {
            Some(router) => router,
            None => {
                self::panic!("Failed to create router");
            }
        };

        let node: Node = Node {
            ip: "localhost".to_string(),
            port: "5433".to_string(),
            name: "test-shard1".to_string(),
        };
        router.configure_shard_connection_to(node);

        // Create a table on the router
        assert!(router
            .send_query("DROP TABLE IF EXISTS test_table;")
            .is_some());
        assert!(router
            .send_query("CREATE TABLE test_table (id INT PRIMARY KEY);")
            .is_some());

        // Insert 10000 rows into the table
        for i in 0..10000 {
            assert!(router
                .send_query(&format!("INSERT INTO test_table VALUES ({});", i))
                .is_some());
        }

        // Select all rows from the table using the shard connection
        let rows = shard_connection
            .query("SELECT * FROM test_table;", &[])
            .unwrap();
        assert_eq!(rows.len(), 10000);

        // Validate the data inserted in each row
        for (i, row) in rows.iter().enumerate() {
            let id: i32 = row.get(0);
            assert_eq!(id, i as i32);
        }

        // Delete half of the rows from the table using the router connection
        assert!(router
            .send_query("DELETE FROM test_table WHERE id % 2 = 0;")
            .is_some());

        // Select all rows from the table using the shard connection
        let rows = shard_connection
            .query("SELECT * FROM test_table;", &[])
            .unwrap();
        assert_eq!(rows.len(), 5000);

        stop_cluster(b"test-shard2\n");
        stop_cluster(b"test-router2\n");
    }

    // Utility functions

    fn create_and_init_cluster(node_name: &[u8], node_type: &str, ip: &str, port: &str) {
        create_cluster_dir(node_name);
        init_cluster(std::str::from_utf8(node_name).unwrap(), node_type);
        wait_for_postgres(ip, port);
    }

    fn create_cluster_dir(node_name: &[u8]) {
        let mut create_cluster = Command::new("./create-cluster-dir.sh")
            .current_dir("..")
            .stdin(Stdio::piped())
            .spawn()
            .expect("failed to create cluster");

        {
            let stdin = create_cluster.stdin.as_mut().expect("failed to open stdin");
            stdin
                .write_all(node_name)
                .expect("failed to write to stdin");
        }

        let create_cluster_status = create_cluster
            .wait()
            .expect("failed to wait on create-cluster-dir.sh");
        if !create_cluster_status.success() {
            self::panic!("create-cluster-dir.sh failed");
        }
    }

    fn init_cluster(node_name: &str, node_type: &str) {
        Command::new("./init-server.sh")
            .current_dir("..")
            .arg(node_name)
            .arg(node_type)
            .arg("&")
            .spawn()
            .expect("failed to start cluster");
    }

    fn stop_cluster(node_name: &[u8]) {
        let mut stop_cluster = Command::new("./server-down.sh")
            .current_dir("..")
            .stdin(Stdio::piped())
            .spawn()
            .expect("failed to stop cluster");

        {
            let stdin = stop_cluster.stdin.as_mut().expect("failed to open stdin");
            stdin
                .write_all(node_name)
                .expect("failed to write to stdin");
        }

        stop_cluster
            .wait()
            .expect("failed to wait on server-down.sh");

        let mut _delete_cluster: std::process::Child = Command::new("rm")
            .current_dir("../clusters")
            .arg("-rf")
            .arg(std::str::from_utf8(node_name).unwrap().trim())
            .spawn()
            .expect("failed to delete cluster");
    }

    fn delete_test_clusters() {
        let clusters_dir = "../clusters";

        for entry in fs::read_dir(clusters_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            // Check if the directory name starts with "test-"
            if path.is_dir()
                && path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("test-")
            {
                let mut _delete_cluster = Command::new("rm")
                    .current_dir(clusters_dir)
                    .arg("-rf")
                    .arg(path.to_str().unwrap())
                    .spawn()
                    .expect("failed to delete cluster");

                _delete_cluster.wait().expect("failed to wait on child");
            }
        }
    }

    fn wait_for_postgres(host: &str, port: &str) {
        let username = match get_current_username() {
            Some(username) => username.to_string_lossy().to_string(),
            None => self::panic!("Failed to get current username"),
        };

        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts > 30 {
                self::panic!("PostgreSQL server did not start in time");
            }

            match Client::connect(
                &format!(
                    "host={} port={} user={} dbname=template1",
                    host, port, username
                ),
                NoTls,
            ) {
                Ok(_) => break,
                Err(_) => std::thread::sleep(std::time::Duration::from_secs(1)),
            }
        }
    }
}
