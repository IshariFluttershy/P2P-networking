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
use tokio::net::{TcpListener, TcpStream, TcpSocket};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use mini_redis::{Connection, Frame, client};
use std::str;
use chrono::{DateTime, Utc, serde::ts_seconds_option, Local};
use std::fs::File;
use std::sync::Arc;
use parking_lot::Mutex;

//use serde::{Deserialize, Serialize};



pub struct NetworkController {
    peers: Arc<Mutex<HashMap<String, Peer>>>,
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
    pub async fn new(
        peers_file: String,
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

        // Listener. Listen to listen_port, accept connections and send a message to rx when accepted
        tokio::spawn( async move {
            loop {
                println!(" DANS LISTENER adresse ip de base: {}", ip_address2.clone());
                let listener = TcpListener::bind(ip_address2.clone() + ":" + &listen_port2).await.unwrap();
                let (socket, socket_addr) = listener.accept().await.unwrap();
                println!(" DANS LISTENER Adresse du socket: {}, adresse ip: {}", socket.local_addr().unwrap().ip(), socket_addr.ip().to_string());
                tx2.send(NetworkControllerEvent::CandidateConnection{
                    ip: socket_addr.ip().to_string(),
                    socket: socket,
                    is_outgoing: false,
                }).await;
            }
        });

        
        let peers_file_clone = peers_file.clone();
        let file = fs::read_to_string(peers_file)?;
        let peers: Arc<Mutex<HashMap<String, Peer>>> = Arc::new(Mutex::new(serde_json::from_str(&file)?));
        
        let mut net = NetworkController{
            peers: Arc::clone(&peers),
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
                thread::sleep(Duration::from_secs(peer_file_dump_interval_seconds));
                let peers_arc = peers.clone();
                let peers_arc = peers_arc.lock();
                let jsons = serde_json::to_string_pretty(&*peers_arc);

                let file = File::create(peers_file_clone.clone());
                file.expect("Cannot write json file").write_all(jsons.expect("Cannot convert peers hashmap as bytes").as_bytes());
            }
        });

        net.start_clients(tx);
        
        Ok(net)
    }


    // Public functions
    pub async fn wait_event(&mut self) -> Result<NetworkControllerEvent::CandidateConnection, &str> {
        self.rx.recv().await.ok_or("Wait_event returns an error")
    }

    pub fn feedback_peer_connected(&mut self, ip: &str, is_outgoing: bool)
    {
        let mut peers = self.peers.lock();
        
        if let None = peers.get_mut(ip) {
            peers.insert(String::from(ip), Peer {
                status: PeerStatus::Idle,
                last_alive: Some(Utc::now()),
                last_failure: None,
                ip: String::from(ip),
            });
        }
        
        if is_outgoing == true {
            peers.get_mut(ip).unwrap().status = PeerStatus::OutHandshaking;
        } else {
            peers.get_mut(ip).unwrap().status = PeerStatus::InHandshaking;
        }
    }
    
    pub async fn feedback_peer_alive(&mut self, ip: &str) {
        let mut peers = self.peers.lock();
        match peers.get(ip).unwrap().status {
            PeerStatus::InHandshaking => peers.get_mut(ip).unwrap().status = PeerStatus::InAlive,
            PeerStatus::OutHandshaking => peers.get_mut(ip).unwrap().status = PeerStatus::OutAlive,
            _ => (),
        }
        peers.get_mut(ip).unwrap().last_alive = Some(Utc::now());
    }
    pub async fn feedback_peer_failed(&mut self, ip: &str) {
        self.peers.lock().get_mut(ip).unwrap().status = PeerStatus::Idle;
        self.peers.lock().get_mut(ip).unwrap().last_failure = Some(Utc::now());
    }
    
    pub fn get_peer_status(&self, ip: &str) -> PeerStatus{
        self.peers.lock().get(ip).unwrap().status.clone()
    }
    

    // Private functions
    fn start_clients(&mut self, tx: Sender<NetworkControllerEvent::CandidateConnection>) {
        for (_, mut peer) in &mut *self.peers.lock() {
            if self.ip_address.eq(&peer.ip) {
                continue;
            }
            let tx_clone = tx.clone();
            peer.status = PeerStatus::OutConnecting;
            let peer_ip = peer.ip.clone();
            let listen_port = self.listen_port.clone();
            let self_ip = self.ip_address.clone();
            
            tokio::spawn( async move {
                start_client(self_ip, peer_ip, listen_port, tx_clone).await;
            });
        }
    }
}

async fn start_client(self_ip: String, target_ip: String, port: String, tx: Sender<NetworkControllerEvent::CandidateConnection>) {
    
    let mut client = connect_client(&self_ip, &target_ip, &port).await;
    tx.send(NetworkControllerEvent::CandidateConnection{
        ip: target_ip,
        socket: client,
        is_outgoing: true,
    }).await;
}

async fn connect_client(self_ip: &str, target_ip: &str, port: &str) -> TcpStream {

    let mut client;
    
    loop {
        println!("client cannot connect, retry in 1 second");
        thread::sleep(Duration::from_secs(1));
        client = get_avaliable_socket(self_ip, port).connect((target_ip.to_owned() + ":" + &port).parse().unwrap()).await;

        match client {
            Ok(_) => break,
            Err(_) => (),
        }
    }

    client.unwrap()
}

fn get_avaliable_socket(self_ip: &str, port: &str) -> TcpSocket {
    let socket = TcpSocket::new_v4().unwrap();
    let mut increment = 0;

    while let Err(_) = socket.bind((self_ip.to_owned() + ":" + &(&port.parse::<i32>().unwrap() + increment).to_string()).parse().unwrap()) {
        increment += 1;
    }

    socket
}