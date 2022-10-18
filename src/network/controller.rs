pub struct NetworkController<'a> {
    peers: HashMap<String, Peer>,
    listen_port: &'a str,
    target_outgoing_connections: u8,
    max_incoming_connections: u8,
    max_simultaneous_outgoing_connection_attempts: u8,
    max_simultaneous_incoming_connection_attempts: u8,
    max_idle_peers: u8,
    max_banned_peers: u8,
    peer_file_dump_interval_seconds: u32,
}

use std::fs;
use std::collections::HashMap;
use std::io;
use super::peer::Peer;

impl NetworkController<'_> {
    pub fn new<'a>(
        peers_file: &'a str,
        listen_port: &'a str,
        target_outgoing_connections: u8,
        max_incoming_connections: u8,
        max_simultaneous_outgoing_connection_attempts: u8,
        max_simultaneous_incoming_connection_attempts: u8,
        max_idle_peers: u8,
        max_banned_peers: u8,
        peer_file_dump_interval_seconds: u32
    ) -> Result<NetworkController, io::Error> {
        
        let file = fs::read_to_string(peers_file)?;
        let peers: HashMap<String, Peer> = serde_json::from_str(&file)?;

        Ok(NetworkController{
            peers: peers,
            listen_port: listen_port,
            target_outgoing_connections: target_outgoing_connections,
            max_incoming_connections: max_incoming_connections,
            max_simultaneous_outgoing_connection_attempts: max_simultaneous_outgoing_connection_attempts,
            max_simultaneous_incoming_connection_attempts: max_simultaneous_incoming_connection_attempts,
            max_idle_peers: max_idle_peers,
            max_banned_peers: max_banned_peers,
            peer_file_dump_interval_seconds: peer_file_dump_interval_seconds
        })
    }
}