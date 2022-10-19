use tokio::net::{TcpListener, TcpStream};
use mini_redis::{Connection, Frame, client, Result};
use std::thread;
use std::time::Duration;
use std::env;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, serde::ts_seconds_option};
use std::fs::File;
use std::fs;
use std::io::prelude::*;
use std::collections::HashMap;

use network::network::controller::NetworkController;
use network::network::controller::NetworkControllerEvent;
use network::network::peer::*;

#[tokio::main]
async fn main() -> Result<()>{
    let args: Vec<String> = env::args().collect();
    let listener_port = args[1].clone();

    let peers_file = "foo.json";
    let listen_port = listener_port;
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
        target_outgoing_connections,
        max_incoming_connections,
        max_simultaneous_outgoing_connection_attempts,
        max_simultaneous_incoming_connection_attempts,
        max_idle_peers,
        max_banned_peers,
        peer_file_dump_interval_seconds
    ).await?;

    tokio::spawn( async move {
        loop {
        tokio::select! {
            evt = net.wait_event() => match evt {
                Ok(msg) => match msg {
                    NetworkControllerEvent::CandidateConnection {ip, socket, is_outgoing} => {
                        if is_outgoing {
                            println!("Connexion sortante acceptée: {:?}", ip)
                        } else {
                            println!("Connexion entrante acceptée: {:?}", ip)
                        }
                        // ip is the peer ip, and socket is a tokio TCPStream
                        // triggered when a new TCP connection with a peer is established
                        // is_outgoing is true if our node has connected to the peer node
                        // is_outgoing is false if the peer node has connected to our node
                        
                        // here, a handshake must be performed by reading/writing data to socket
                        //  if the handshake succesds, call net.feedback_peer_alive(ip).await; to signal NetworkController to set the peer in InAlive or OutAlive state (this should update last_alive)
                        //  if handshake fails or the connection closes unexpectedly at any time, call net.feedback_peer_failed(ip).await; to signal NetworkController to set the peer status to Idle  (this should update last_failure)
                        
                        // once the handshake is done, we can use this peer socket in main.rs
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