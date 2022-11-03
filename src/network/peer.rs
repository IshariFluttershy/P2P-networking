use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, serde::ts_seconds_option};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, Write};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PeerStatus {
    Idle,
    OutConnecting, 
    OutHandshaking, 
    OutAlive, 
    InHandshaking, 
    InAlive, 
    Banned,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Peer {
    pub status: PeerStatus,
    #[serde(with = "ts_seconds_option")]
    pub last_alive: Option<DateTime<Utc>>,
    #[serde(with = "ts_seconds_option")]
    pub last_failure: Option<DateTime<Utc>>,
    pub ip: String,
}

// Utility function that creates a default json file with two peers
pub fn create_new_peers_json_file(path: &str) -> Result<(), Error>{
    let peer = Peer {
        status: PeerStatus::Idle,
        last_alive: None,
        last_failure: None,
        ip: String::from("127.0.0.1:555"),
    };

    let peer2 = Peer {
        status: PeerStatus::Idle,
        last_alive: Some(Utc::now()),
        last_failure: Some(Utc::now()),
        ip: String::from("127.0.0.1:666"),
    };

    let mut peers = HashMap::new();
    peers.insert(peer.ip.clone(), peer);
    peers.insert(peer2.ip.clone(), peer2);

    let json = serde_json::to_string_pretty(&peers)?;

    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}
