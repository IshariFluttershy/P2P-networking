pub mod NetworkControllerEvent {

    use tokio::net::TcpStream;

    #[derive(Debug)]
    pub struct CandidateConnection {
        pub ip: String,
        pub socket: TcpStream,
        pub is_outgoing: bool
    }
}

use std::fs;
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Write};
use super::peer::{Peer, PeerStatus};
use std::time::Duration;
use std::thread;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use mini_redis::{Connection, Frame, client};
use std::str;
use chrono::{DateTime, Utc, serde::ts_seconds_option};
use std::fs::File;

//use serde::{Deserialize, Serialize};



pub struct NetworkController {
    peers: HashMap<String, Peer>,
    listen_port: String,
    ip_address: String,
    target_outgoing_connections: u8,
    max_incoming_connections: u8,
    max_simultaneous_outgoing_connection_attempts: u8,
    max_simultaneous_incoming_connection_attempts: u8,
    max_idle_peers: u8,
    max_banned_peers: u8,
    peer_file_dump_interval_seconds: u64,   
    rx: Receiver<NetworkControllerEvent::CandidateConnection>,
}


impl NetworkController {
    pub async fn new<'a>(
        peers_file: &'a str,
        listen_port: String,
        ip_address: String,
        target_outgoing_connections: u8,
        max_incoming_connections: u8,
        max_simultaneous_outgoing_connection_attempts: u8,
        max_simultaneous_incoming_connection_attempts: u8,
        max_idle_peers: u8,
        max_banned_peers: u8,
        peer_file_dump_interval_seconds: u64
    ) -> Result<NetworkController, Error> {

        let (tx, rx) = mpsc::channel(32);
        let tx2 = tx.clone();
        let listen_port2 = listen_port.clone();
        let ip_address2 = ip_address.clone();

        // Listener
        tokio::spawn( async move {
            loop {
                let listener = TcpListener::bind(ip_address2.clone() + ":" + &listen_port2).await.unwrap();
                let (socket, socket_addr) = listener.accept().await.unwrap();
                tx2.send(NetworkControllerEvent::CandidateConnection{
                    ip: socket_addr.ip().to_string(),
                    socket: socket,
                    is_outgoing: false,
                }).await;
            }
        });

        
        let file = fs::read_to_string(peers_file)?;
        let peers: HashMap<String, Peer> = serde_json::from_str(&file)?;
        
        let mut net = NetworkController{
            peers: peers,
            listen_port: listen_port,
            ip_address: ip_address,
            target_outgoing_connections: target_outgoing_connections,
            max_incoming_connections: max_incoming_connections,
            max_simultaneous_outgoing_connection_attempts: max_simultaneous_outgoing_connection_attempts,
            max_simultaneous_incoming_connection_attempts: max_simultaneous_incoming_connection_attempts,
            max_idle_peers: max_idle_peers,
            max_banned_peers: max_banned_peers,
            peer_file_dump_interval_seconds: peer_file_dump_interval_seconds,
            rx: rx
        };
        
        tokio::spawn( async move {
            loop {
                thread::sleep(Duration::from_secs(net.peer_file_dump_interval_seconds));
                let jsons = serde_json::to_string_pretty(&net.peers);

                let file = File::create(peers_file);
                file.expect("Cannot write json file").write_all(jsons.expect("Cannot convert peers hashmap as bytes").as_bytes());
            }
        });

        net.start_clients(tx);
        
        Ok(net)
    }

    pub async fn wait_event(&mut self) -> Result<NetworkControllerEvent::CandidateConnection, &str> {
        self.rx.recv().await.ok_or("Wait_event returns an error")
    }

    
    fn start_clients(&mut self, tx: Sender<NetworkControllerEvent::CandidateConnection>) {
        for (_, mut peer) in &mut self.peers {
            if self.ip_address.eq(&peer.ip) {
                continue;
            }
            let tx_clone = tx.clone();
            peer.status = PeerStatus::OutConnecting;
            let peer_ip = peer.ip.clone();
            let listen_port = self.listen_port.clone();
            
            tokio::spawn( async move {
                start_client(peer_ip, listen_port, tx_clone).await;
            });
        }
    }

    pub fn feedback_peer_connected(&mut self, ip: &str, is_outgoing: bool)
    {
        if is_outgoing == true {
            self.peers.get_mut(ip).unwrap().status = PeerStatus::OutHandshaking;
        } else {
            self.peers.get_mut(ip).unwrap().status = PeerStatus::InHandshaking;
        }
    }

    pub async fn feedback_peer_alive(&mut self, ip: &str) {
        match self.peers.get(ip).unwrap().status {
            PeerStatus::InHandshaking => self.peers.get_mut(ip).unwrap().status = PeerStatus::InAlive,
            PeerStatus::OutHandshaking => self.peers.get_mut(ip).unwrap().status = PeerStatus::OutAlive,
            _ => (),
        }
        self.peers.get_mut(ip).unwrap().last_alive = Some(Utc::now())
    }
    pub async fn feedback_peer_failed(&mut self, ip: &str) {
        self.peers.get_mut(ip).unwrap().status = PeerStatus::Idle;
        self.peers.get_mut(ip).unwrap().last_failure = Some(Utc::now())
    }

    pub fn get_peer_status(&self, ip: &str) -> PeerStatus{
        self.peers.get(ip).unwrap().status.clone()
    }
}

async fn start_client(ip: String, port: String, tx: Sender<NetworkControllerEvent::CandidateConnection>) {
    
    let mut client = connect_client(&ip, &port).await;
    tx.send(NetworkControllerEvent::CandidateConnection{
        ip: ip,
        socket: client,
        is_outgoing: true,
    }).await;
}

async fn connect_client(ip: &str, port: &str) -> TcpStream {
    let mut client = TcpStream::connect(ip.to_owned() + ":" + &port).await;
    
    while let Err(_e) = client {
        println!("client cannot connect, retry in 1 second");
        thread::sleep(Duration::from_secs(1));
        client = TcpStream::connect(ip.to_owned() + ":" + &port).await;
    }
    client.unwrap()
}




