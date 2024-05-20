use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use crate::generic_stream::GenericStream;

pub type Clients = HashMap<ClientId, Client>;

pub trait ClientDatabase {
    fn remove_user(&mut self, id: &ClientId);
    fn get_stream(&self, id: &ClientId) -> Arc<GenericStream>;
    fn to_html_string(&self) -> String;
    fn update_battle_status(&mut self, m_id: &ClientId, p_id: &ClientId, status: bool);
    fn remove_battle_status(&mut self, m_id: &ClientId);
    fn get_ids(&self) -> String;
}

impl ClientDatabase for Clients {
    fn remove_user(&mut self, id: &ClientId) {
        self.remove(id);
    }

    fn get_stream(&self, id: &ClientId) -> Arc<GenericStream> {
        Arc::clone(
            &self
                .get(id)
                .unwrap_or_else(|| panic!("id {id} not found"))
                .stream,
        )
    }
    fn to_html_string(&self) -> String {
        self.iter()
            .map(|(id, client)| {
                let id = to_hex_str(id);
                if client.is_battling {
                    format!("<li> {id} ⚔️</li>")
                } else {
                    format!("<li> {id} </li>")
                }
            })
            .collect::<Vec<String>>()
            .join("")
    }

    fn update_battle_status(&mut self, m_id: &ClientId, p_id: &ClientId, status: bool) {
        if let Some(player) = self.get_mut(m_id) {
            player.set_battling(status);
        }
        if let Some(player) = self.get_mut(p_id) {
            player.set_battling(status);
        }
    }

    fn remove_battle_status(&mut self, m_id: &ClientId) {
        if let Some(player) = self.get_mut(m_id) {
            player.set_battling(false);
        }
    }

    fn get_ids(&self) -> String {
        self.iter()
            .map(|(id, _)| to_hex_str(id))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

pub type ClientId = u16;

pub struct Client {
    password: Vec<u8>,
    pub stream: Arc<GenericStream>,
    is_battling: bool,
}

impl Client {
    pub fn new(password: Vec<u8>, stream: GenericStream) -> Self {
        Client {
            password,
            stream: Arc::new(stream),
            is_battling: false,
        }
    }

    pub fn check_password(&self, password: &Vec<u8>) -> bool {
        password.eq(&self.password)
    }

    pub fn set_battling(&mut self, is_battling: bool) {
        self.is_battling = is_battling;
    }
}

pub fn get_hash<T: Hash>(t: &T) -> u16 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish() as u16
}

pub fn to_hex_str(id: &ClientId) -> String {
    let u8s: [u8; 2] = id.to_be_bytes();
    format!("{:02x}{:02x}", u8s[0], u8s[1])
}
