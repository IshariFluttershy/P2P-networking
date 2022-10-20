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
use std::io;
use super::peer::Peer;
use std::time::Duration;
use std::thread;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use mini_redis::{Connection, Frame, client};

pub struct NetworkController {
    peers: HashMap<String, Peer>,
    listen_port: String,
    target_outgoing_connections: u8,
    max_incoming_connections: u8,
    max_simultaneous_outgoing_connection_attempts: u8,
    max_simultaneous_incoming_connection_attempts: u8,
    max_idle_peers: u8,
    max_banned_peers: u8,
    peer_file_dump_interval_seconds: u32,   
    rx: Receiver<NetworkControllerEvent::CandidateConnection>,
}


impl NetworkController {
    pub async fn new<'a>(
        peers_file: &'a str,
        listen_port: String,
        target_outgoing_connections: u8,
        max_incoming_connections: u8,
        max_simultaneous_outgoing_connection_attempts: u8,
        max_simultaneous_incoming_connection_attempts: u8,
        max_idle_peers: u8,
        max_banned_peers: u8,
        peer_file_dump_interval_seconds: u32
    ) -> Result<NetworkController, io::Error> {

        let (tx, rx) = mpsc::channel(32);
        let tx2 = tx.clone();
        let listen_port2 = listen_port.clone();

        // Listener
        tokio::spawn( async move {
            loop {
                let listener = TcpListener::bind("127.0.0.1:".to_owned() + &listen_port2).await.unwrap();
                let (socket, socket_addr) = listener.accept().await.unwrap();
                tx2.send(NetworkControllerEvent::CandidateConnection{
                    ip: socket_addr.ip().to_string() + &socket_addr.port().to_string(),
                    socket: socket,
                    is_outgoing: false,
                }).await;
            }
        });

        let file = fs::read_to_string(peers_file)?;
        let peers: HashMap<String, Peer> = serde_json::from_str(&file)?;
        
        let net = NetworkController{
            peers: peers,
            listen_port: listen_port,
            target_outgoing_connections: target_outgoing_connections,
            max_incoming_connections: max_incoming_connections,
            max_simultaneous_outgoing_connection_attempts: max_simultaneous_outgoing_connection_attempts,
            max_simultaneous_incoming_connection_attempts: max_simultaneous_incoming_connection_attempts,
            max_idle_peers: max_idle_peers,
            max_banned_peers: max_banned_peers,
            peer_file_dump_interval_seconds: peer_file_dump_interval_seconds,
            rx: rx
        };

        net.start_clients(tx2);
        
        Ok(net)
    }

    pub async fn wait_event(&mut self) -> Result<NetworkControllerEvent::CandidateConnection, &str> {
        self.rx.recv().await.ok_or("Wait_event returns an error")
    }

    
    fn start_clients(&self, tx: Sender<NetworkControllerEvent::CandidateConnection>) {
        for (_, peer) in &self.peers {
            let tx_clone = tx.clone();
            let peer_clone = peer.ip.clone();
            
            tokio::spawn( async move {
                start_client(peer_clone, tx_clone).await;
            });
        }
    }
}

async fn start_client(socket: String, tx: Sender<NetworkControllerEvent::CandidateConnection>) {
    let mut client = connect_client(&socket).await;
    tx.send(NetworkControllerEvent::CandidateConnection{
        ip: socket,
        socket: client,
        is_outgoing: true,
    }).await;
    /*client.set("hello", port.clone().into()).await.unwrap();
    
    loop {
        let result = client.get("hello").await;
        match result {
            Ok(t) => println!("got value from the server; result={:?}", t),
            Err(e) => {println!("cannot get value from the server; error={:?}", e);
            client = connect_client(&("127.0.0.1:".to_owned() + &port)).await;
            client.set("hello", port.clone().into()).await.unwrap();},
        }
        thread::sleep(Duration::from_secs(1));
    }*/
}

async fn connect_client(socket: &str) -> TcpStream {
    let mut client = TcpStream::connect(socket).await;
    
    while let Err(_e) = client {
        println!("client cannot connect, retry in 1 second");
        thread::sleep(Duration::from_secs(1));
        client = TcpStream::connect(socket).await;
    }
    client.unwrap()
}


async fn process(socket: TcpStream) {
    use mini_redis::Command::{self, Get, Set};

    // A hashmap is used to store data
    let mut db = HashMap::new();

    // Connection, provided by `mini-redis`, handles parsing frames from
    // the socket
    let mut connection = Connection::new(socket);

    // Use `read_frame` to receive a command from the connection.
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                // The value is stored as `Vec<u8>`
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    // `Frame::Bulk` expects data to be of type `Bytes`. This
                    // type will be covered later in the tutorial. For now,
                    // `&Vec<u8>` is converted to `Bytes` using `into()`.
                    Frame::Bulk(value.clone().into())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        // Write the response to the client
        connection.write_frame(&response).await.unwrap();
    }
}

