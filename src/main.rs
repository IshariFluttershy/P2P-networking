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

#[derive(Serialize, Deserialize, Debug)]
enum PeerStatus {
    Idle,
    OutConnecting, 
    OutHandshaking, 
    OutAlive, 
    InHandshaking, 
    InAlive, 
    Banned,
}

#[derive(Serialize, Deserialize, Debug)]
struct Peer {
    status: PeerStatus,
    #[serde(with = "ts_seconds_option")]
    last_alive: Option<DateTime<Utc>>,
    #[serde(with = "ts_seconds_option")]
    last_failure: Option<DateTime<Utc>>,
    ip: String,
}

#[tokio::main]
async fn main() -> Result<()>{
    let args: Vec<String> = env::args().collect();
    let listener_port = args[1].clone();
    let client_port = args[2].clone();
    let client_port2 = args[3].clone();

    let peer = Peer {
        status: PeerStatus::Idle,
        last_alive: Some(Utc::now()),
        last_failure: Some(Utc::now()),
        ip: String::from("127.0.0.1:555"),
    };

    let peer2 = Peer {
        status: PeerStatus::Idle,
        last_alive: Some(Utc::now()),
        last_failure: Some(Utc::now()),
        ip: String::from("127.0.0.1:666"),
    };

    let json1 = serde_json::to_string_pretty(&peer)?;
    let json2 = serde_json::to_string_pretty(&peer)?;

    let mut jsons = HashMap::new();
    jsons.insert(peer.ip.clone(), peer);
    jsons.insert(peer2.ip.clone(), peer2);

    let jsons = serde_json::to_string_pretty(&jsons)?;

    let mut file = File::create("foo.txt")?;
    file.write_all(jsons.as_bytes())?;


    let file = fs::read_to_string("foo.txt")?;
    let deserialized_peers : HashMap<&str, Peer> = serde_json::from_str(&file)?;

    println!("{:?}", deserialized_peers);





    // Bind the listener to the address
    tokio::spawn( async move {
        
        loop {
            let listener = TcpListener::bind("127.0.0.1:".to_owned() + &listener_port).await.unwrap();
            println!("Ca bosse dans le thread");
            let (socket, _) = listener.accept().await.unwrap();
            println!("Ca bosse toujours dans le thread");
            tokio::spawn( async move {
                process(socket).await;
                println!("Ca bosse encore et toujours dans le thread");
            });
        }
    });

    thread::sleep(Duration::from_secs(1));

    tokio::spawn( async move {
        start_client(client_port).await;
    });
    tokio::spawn( async move {
        start_client(client_port2).await;
    });

    loop{}

    Ok(())
}

async fn connect_client(addr: &str) -> client::Client {
    let mut client = client::connect(addr).await;

    while let Err(_e) = client {
        println!("client cannot connect, retry in 1 second");
        thread::sleep(Duration::from_secs(1));
        client = client::connect(addr).await;
    }
    client.unwrap()
}

async fn start_client(port: String) {
    let mut client = connect_client(&("127.0.0.1:".to_owned() + &port)).await;
    client.set("hello", port.clone().into()).await.unwrap();

    loop {
        let result = client.get("hello").await;
        match result {
            Ok(t) => println!("got value from the server; result={:?}", t),
            Err(e) => {println!("cannot get value from the server; error={:?}", e);
                client = connect_client(&("127.0.0.1:".to_owned() + &port)).await;
            client.set("hello", port.clone().into()).await.unwrap();},
        }
        thread::sleep(Duration::from_secs(1));
    }
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