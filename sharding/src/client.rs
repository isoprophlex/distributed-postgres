use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    match TcpStream::connect("127.0.0.1:7878") {
        Ok(mut stream) => {
            println!("Connected to server on port 7878");

            let msg = b"Hello from client!!";
            stream.write(msg)?;
            println!("Sent: {}", String::from_utf8_lossy(msg));

            let mut buffer = [0; 512];
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            match stream.read(&mut buffer) {
                Ok(_) => {
                    println!("Response from server: {}", String::from_utf8_lossy(&buffer[..]));
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Error connecting with server: {}", e);
        }
    }
    Ok(())
}