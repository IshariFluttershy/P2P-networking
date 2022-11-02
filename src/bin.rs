use tokio::net::{TcpListener, TcpStream};
use mini_redis::{Connection, Frame, client};
use std::thread;
use std::time::Duration;
use std::env;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, serde::ts_seconds_option};
use std::fs::File;
use std::fs;
use std::io::prelude::*;
use std::io;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::str;



use network::network::controller::NetworkController;
use network::network::controller::NetworkControllerEvent;
use network::network::peer::*;

#[tokio::main]
async fn main() -> Result<(), Error>{
    let args: Vec<String> = env::args().collect();
    let ip_addr = args[1].clone();

    let peers_file = String::from("foo.json");
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

    tokio::spawn( async move {
        loop {
        tokio::select! {
            evt = net.wait_event() => match evt {
                Ok(msg) => match msg {
                    NetworkControllerEvent::CandidateConnection {ip, socket, is_outgoing} => {
                        net.feedback_peer_connected(&ip, is_outgoing);
                        let result;
                        if is_outgoing {
                            result = try_to_handshake(&socket).await;
                        } else {
                            result = listen_to_handshake(&socket).await;
                        }
                        
                        match result {
                            Ok(()) => net.feedback_peer_alive(&ip).await,
                            _ => net.feedback_peer_failed(&ip).await,
                        }
                        println!("État du peer {}: {:?}", socket.peer_addr().unwrap().ip(), net.get_peer_status(&ip));
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

pub async fn try_to_handshake(socket: &TcpStream) -> Result<(), Error> {
    loop {
        socket.writable().await;
        match socket.try_write(b"handshake") {
            Ok(n) => {
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

pub async fn listen_to_handshake(socket: &TcpStream) -> Result<(), Error> {
    let mut buf = [0; 4096];
    loop {
        socket.readable().await?;

        match socket.try_read(&mut buf) {
            Ok(0) => return Err(Error::new(ErrorKind::Other, "handshake failed")),
            Ok(n) => {
                let result = str::from_utf8(&buf[0..n]).unwrap();
                println!("Read buffer : {:?}", str::from_utf8(&buf[0..n]).unwrap());
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