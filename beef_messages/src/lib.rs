//! Simple custom protocol for interacting with server
//!
//! beef messages:
//! ```plaintext
//! ┏━━━━━━━━━━┳━━━━━━━┳━━━━━━━━━┓
//! ┃  command ┃ c_id  ┃ payload ┃
//! ┗━━━━━━━━━━┻━━━━━━━┻━━━━━━━━━┛
//!
//! ┃    u8    ┃  u16? ┃  n*u8?  ┃
//! ```
//! Every message contains command bytes, and - if needed for the command - the client_id (`c_id`),
//! and the payload - which can be variable in length. We don't need any lengths as the structure
//! for each command is known and well-defined.

pub type Payload = Vec<u8>;
pub type ClientId = u16;
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BeefMessage {
    /// Lists all users online
    List,
    /// Initiates battle against user with id [ClientId] with the target word [Payload]
    BattleInit(ClientId, Payload),
    /// Guesses the word [Payload], to be compared against the target word
    BattleGuess(Payload),
    /// Forfeits the current battle
    BattleForfeit,
    /// Sends [Payload] to player guessing words
    Message(Payload),
    /// Gracefully disconnect from server
    Disconnect,
    /// Malformed commands
    NotBeef,
}

impl From<Vec<u8>> for BeefMessage {
    fn from(value: Vec<u8>) -> Self {
        if value.is_empty() {
            return BeefMessage::NotBeef;
        };

        return match value.first().unwrap() {
            0x6c => BeefMessage::List,
            0x62 => {
                let length = value.len();
                if length < 4 {
                    return BeefMessage::NotBeef;
                }
                let client_id_bytes: (u8, u8) = (value[1], value[2]);
                BeefMessage::BattleInit(
                    parse_client_id(client_id_bytes),
                    value[3..length].to_vec(),
                )
            }
            0x66 => BeefMessage::BattleForfeit,
            0x67 => {
                let length = value.len();
                if length < 2 {
                    return BeefMessage::NotBeef;
                }
                BeefMessage::BattleGuess(value[1..length].to_vec())
            }
            0x64 => BeefMessage::Disconnect,
            0x6d => {
                let length = value.len();
                if length < 2 {
                    return BeefMessage::NotBeef;
                }
                BeefMessage::Message(value[1..length].to_vec())
            }
            _ => BeefMessage::NotBeef,
        };
    }
}

impl From<BeefMessage> for Vec<u8> {
    fn from(val: BeefMessage) -> Vec<u8> {
        match val {
            BeefMessage::List => {
                vec![0x6c]
            }
            BeefMessage::BattleInit(id, payload) => {
                let mut command: Vec<u8> = vec![0x62];
                command.push(*id.to_be_bytes().first().unwrap());
                command.push(*id.to_be_bytes().last().unwrap());
                command.append(&mut payload.to_vec());
                command
            }
            BeefMessage::BattleGuess(payload) => {
                let mut command: Vec<u8> = vec![0x67];
                command.append(&mut payload.to_vec());
                command
            }
            BeefMessage::BattleForfeit => {
                vec![0x66]
            }
            BeefMessage::Disconnect => {
                vec![0x64]
            }
            BeefMessage::Message(payload) => {
                let mut command: Vec<u8> = vec![0x6d];
                command.append(&mut payload.to_vec());
                command
            }
            BeefMessage::NotBeef => {
                vec![0xff]
            }
        }
    }
}

impl BeefMessage {
    pub fn get_payload(&self) -> Option<Payload> {
        match self {
            BeefMessage::BattleInit(_, p) => { Some(p.clone())}
            BeefMessage::BattleGuess(p) => { Some(p.clone())}
            BeefMessage::Message(p) => { Some(p.clone())}
            _ => { None }
        }
    }
}

// take first byte to u16 and shift left 8 and OR it with second byte
fn parse_client_id(client_id_bytes: (u8, u8)) -> u16 {
    ((client_id_bytes.0 as u16) << 8) | client_id_bytes.1 as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_ser_deser() {
        let beef_msg = BeefMessage::List;
        let beef_ser: Vec<u8> = beef_msg.clone().into();
        let beef_ser_deser: BeefMessage = beef_ser.clone().into();
        assert_eq!(beef_msg, beef_ser_deser);

        let beef_msg = BeefMessage::NotBeef;
        let beef_ser: Vec<u8> = beef_msg.clone().into();
        let beef_ser_deser: BeefMessage = beef_ser.clone().into();
        assert_eq!(beef_msg, beef_ser_deser);

        let beef_msg = BeefMessage::BattleForfeit;
        let beef_ser: Vec<u8> = beef_msg.clone().into();
        let beef_ser_deser: BeefMessage = beef_ser.clone().into();
        assert_eq!(beef_msg, beef_ser_deser);

        let beef_msg = BeefMessage::Disconnect;
        let beef_ser: Vec<u8> = beef_msg.clone().into();
        let beef_ser_deser: BeefMessage = beef_ser.clone().into();
        assert_eq!(beef_msg, beef_ser_deser);

    }

    #[test]
    fn payload_ser_deser() {
        let beef_msg = BeefMessage::Message([0xabu8, 0xaau8].to_vec());
        let beef_ser: Vec<u8> = beef_msg.clone().into();
        let beef_ser_deser: BeefMessage = beef_ser.clone().into();
        assert_eq!(beef_msg, beef_ser_deser);

        let beef_msg = BeefMessage::BattleGuess([0xabu8, 0xaau8].to_vec());
        let beef_ser: Vec<u8> = beef_msg.clone().into();
        let beef_ser_deser: BeefMessage = beef_ser.clone().into();
        assert_eq!(beef_msg, beef_ser_deser);
    }

    #[test]
    fn client_payload_ser_deser() {
        let beef_msg = BeefMessage::BattleInit(0x1234u16,[0xabu8, 0xaau8].to_vec());
        let beef_ser: Vec<u8> = beef_msg.clone().into();
        let beef_ser_deser: BeefMessage = beef_ser.clone().into();
        assert_eq!(beef_msg, beef_ser_deser);
    }
}
