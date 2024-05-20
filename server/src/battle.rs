use std::collections::HashSet;

use crate::client::ClientId;

pub type Battles = HashSet<Battle>;
pub type BattleId = (ClientId, ClientId);

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Battle {
    pub id: BattleId,
    pub target: String,
    previous_guesses: Vec<String>,
}

impl Battle {
    pub fn new(master: ClientId, player: ClientId, target: String) -> Self {
        Battle {
            id: (master, player),
            target,
            previous_guesses: Vec::new(),
        }
    }
    fn to_html_string(&self) -> String {
        format!("<div style=\"border: 1px dotted #dddddd; margin: 1em; padding: 1em; max-width: 30em;\">
            <span><strong>#{:?}</strong> vs. <strong>#{:?}</strong></span><br><hr>
            <span>🡒 <strong>{}</strong></span><span> 🡐 </span><span>{}</span>
        </div>",
                self.id.0, self.id.1, self.target.trim(), self.previous_guesses())
    }

    pub fn check_guess(&mut self, guess: String) -> bool {
        if self.target.eq(&guess) {
            return true;
        }
        self.previous_guesses.push(guess);
        false
    }

    fn previous_guesses(&self) -> String {
        self.previous_guesses
            .iter()
            .map(|guess| guess.trim().to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }

    pub fn get_opponnent(&self, my_id: &ClientId) -> ClientId {
        if self.id.0.eq(my_id) {
            self.id.1
        } else {
            self.id.0
        }
    }
}

pub trait BattleDatabase {
    fn add_battle(&mut self, battle: Battle);
    fn del_battle(&mut self, battle_id: &BattleId);
    fn del_battle_by_client(&mut self, client_id: &ClientId) -> Option<ClientId>;
    fn to_html_string(&self) -> String;
    fn update_or_add_battle(&mut self, battle: Battle);
    fn exists_by_id(&self, client_id: &ClientId) -> bool;
    fn get_current_battle(&self, id: &ClientId) -> Option<Battle>;
}

impl BattleDatabase for Battles {
    fn add_battle(&mut self, battle: Battle) {
        self.insert(battle);
    }

    fn del_battle(&mut self, battle_id: &BattleId) {
        self.retain(|battle| !battle.id.eq(battle_id));
    }
    fn del_battle_by_client(&mut self, client_id: &ClientId) -> Option<ClientId> {
        if let Some(battle_found) = self
            .iter()
            .find(|&battle| battle.id.0.eq(client_id) || battle.id.1.eq(client_id))
            .cloned()
        {
            self.retain(|battle| !battle.id.0.eq(client_id) && !battle.id.1.eq(client_id));
            let other_id = battle_found.get_opponnent(client_id);
            return Some(other_id);
        }
        None
    }

    fn to_html_string(&self) -> String {
        self.iter()
            .map(|battle| battle.to_html_string())
            .collect::<Vec<String>>()
            .join("")
    }

    fn update_or_add_battle(&mut self, battle: Battle) {
        self.del_battle(&battle.id);
        self.add_battle(battle);
    }

    fn exists_by_id(&self, client_id: &ClientId) -> bool {
        self.iter()
            .any(|battle| battle.id.0.eq(client_id) || battle.id.0.eq(client_id))
    }

    fn get_current_battle(&self, client_id: &ClientId) -> Option<Battle> {
        self.iter()
            .find(|&battle| battle.id.1.eq(client_id) || battle.id.0.eq(client_id))
            .cloned()
    }
}
