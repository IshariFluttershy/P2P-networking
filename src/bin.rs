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