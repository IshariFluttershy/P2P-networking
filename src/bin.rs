use tokio::net::{TcpStream};
use std::env;
use std::io;
use std::io::{Error, ErrorKind};
use std::str;
use network::network::controller::NetworkController;
use network::network::controller::NetworkControllerEvent;


// Start the program with "cargo run IP_ADDRESS" with IP_ADDRESS an IPV4 valid address. It will set the IP address of the program with IP_ADDRESS.
#[tokio::main]
async fn main() -> Result<(), Error>{
    let args: Vec<String> = env::args().collect();
    let ip_addr = args[1].clone();

    let peers_file = String::from(ip_addr.clone() + ".json");
    let listen_port = String::from("6380");
    let target_outgoing_connections = 5;
    let max_incoming_connections = 5;
    let max_simultaneous_outgoing_connection_attempts = 5;
    let max_simultaneous_incoming_connection_attempts = 5;
    let max_idle_peers = 5;
    let max_banned_peers = 5;
    let peer_file_dump_interval_seconds = 5;

    let mut net = NetworkController::new(
        peers_file,
        listen_port,
        ip_addr,
        target_outgoing_connections,
        max_incoming_connections,
        max_simultaneous_outgoing_connection_attempts,
        max_simultaneous_incoming_connection_attempts,
        max_idle_peers,
        max_banned_peers,
        peer_file_dump_interval_seconds
    ).await?;

    // Async loop to listen on net events. When a CandidateConnection is raised, a handcheck is performed.
    tokio::spawn( async move {
        loop {
        tokio::select! {
            evt = net.wait_event() => match evt {
                Ok(msg) => match msg {
                    NetworkControllerEvent::CandidateConnection {ip, socket, is_outgoing} => {
                        println!("New candidate connection. ip: {}, is_outgoing: {}", ip, is_outgoing);
                        net.feedback_peer_connected(&ip, is_outgoing);

                        let result;
                        if is_outgoing {
                            result = try_to_handshake(&socket).await;
                        } else {
                            result = listen_to_handshake(&socket).await;
                        }

                        match result {
                            Ok(()) => {
                                println!("Handshake success");
                                net.feedback_peer_alive(&ip).await;
                                },
                            _ => {
                                println!("Handshake failed");
                                net.feedback_peer_failed(&ip).await;
                            },
                        }
                    }
                },
                Err(e) => println!("Ca renvoie une erreur: {}", e),
            }
        }
    }
    });

    loop{}

    Ok(())
}

// TODO: Outgoing handshake must listen and wait for response
// Handshake function when connection is outgoing. 
pub async fn try_to_handshake(socket: &TcpStream) -> Result<(), Error> {
    loop {
        socket.writable().await.unwrap();
        match socket.try_write(b"handshake") {
            Ok(_) => {
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    } 

    Ok(())
}

// TODO: Incoming handshake must write and send a response
// Handshake function when connection is incoming. 
pub async fn listen_to_handshake(socket: &TcpStream) -> Result<(), Error> {
    let mut buf = [0; 4096];
    loop {
        socket.readable().await?;

        match socket.try_read(&mut buf) {
            Ok(0) => return Err(Error::new(ErrorKind::Other, "handshake failed")),
            Ok(n) => {
                let result = str::from_utf8(&buf[0..n]).unwrap();
                if result.eq("handshake") {
                    return Ok(());
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}