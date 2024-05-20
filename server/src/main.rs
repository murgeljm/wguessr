extern crate beef_messages;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex, MutexGuard};
use std::{fs, thread};

use beef_messages::{BeefMessage, Payload};

use crate::battle::{Battle, BattleDatabase, BattleId, Battles};
use crate::client::get_hash;
use crate::client::{Client, ClientDatabase, ClientId, Clients};
use crate::generic_stream::GenericStream;
use crate::http::serve_info_site;

mod battle;
mod client;
mod generic_stream;
mod http;

fn main() -> std::io::Result<()> {
    // global data stores
    let clients = Arc::new(Mutex::new(Clients::new()));
    let battles = Arc::new(Mutex::new(Battles::new()));

    let server_addr = "127.0.0.1:1234";
    let socket_path = "/tmp/guess_a_word.socket";

    /*
        using futures we could just turn iters into streams and merge them, without the need
        for two separate threads and arc::clones for them, but I wanted 0 dependencies declared
        in toml, even if the dependency is from the rust team itself
    */
    let clients_ref = Arc::clone(&clients);
    let battles_ref = Arc::clone(&battles);
    let tcp_thread = thread::spawn(move || {
        let listener = TcpListener::bind(server_addr).unwrap();
        for stream in listener.incoming() {
            let stream = Box::new(GenericStream::TcpStream(stream.unwrap()));
            let clients = Arc::clone(&clients_ref);
            let battles = Arc::clone(&battles_ref);
            thread::spawn(move || match handle_connection(stream, clients, battles) {
                Ok(_) => {}
                Err(_) => {
                    println!("stream closed")
                }
            });
        }
    });

    // handle Unix streams (cleanup if needed)
    match fs::remove_file(socket_path) {
        Ok(_) => {}
        Err(error) => {
            println!("{}", error)
        }
    }
    let clients_ref = Arc::clone(&clients);
    let battles_ref = Arc::clone(&battles);
    let unix_thread = thread::spawn(move || {
        let listener = UnixListener::bind(socket_path).unwrap();
        for stream in listener.incoming() {
            let stream = Box::new(GenericStream::UnixStream(stream.unwrap()));
            let clients = Arc::clone(&clients_ref);
            let battles = Arc::clone(&battles_ref);

            thread::spawn(move || match handle_connection(stream, clients, battles) {
                Ok(_) => {}
                Err(_) => {
                    println!("stream closed")
                }
            });
        }
    });

    println!(
        "\nserver online! connect with a client or visit localhost:1234 for a web interface\n"
    );

    tcp_thread.join().unwrap();
    unix_thread.join().unwrap();
    Ok(())
}

fn handle_connection(
    stream: Box<GenericStream>,
    clients: Arc<Mutex<Clients>>,
    battles: Arc<Mutex<Battles>>,
) -> Result<(), ()> {
    if let Some(client_id) = check_client(stream, &clients, &battles) {
        let client_stream = clients.lock().unwrap().get_stream(&client_id);
        loop {
            let msg = client_stream.receive_msg()?;
            match msg {
                BeefMessage::List => {
                    let out_stream = clients.lock().unwrap().get_stream(&client_id);
                    let clients = clients.lock().unwrap().get_ids();
                    out_stream.send_msg_string(format!("beef: USERS ONLINE:\n{}", clients));
                }
                BeefMessage::BattleInit(to_id, target) => {
                    let Ok(new_battle) = start_new_beef(
                        client_id,
                        to_id,
                        target,
                        &client_stream,
                        clients.lock().unwrap(),
                        battles.lock().unwrap(),
                    ) else {
                        continue;
                    };
                    battles.lock().unwrap().add_battle(new_battle);
                    clients
                        .lock()
                        .unwrap()
                        .update_battle_status(&client_id, &to_id, true);
                    client_stream
                        .send_msg_string(format!("beef: STARTING BEEF WITH USER {to_id:x}!"));
                    clients
                        .lock()
                        .unwrap()
                        .get_stream(&to_id)
                        .send_msg_string(format!(
                            "beef: USER {client_id:x} HAS BEEF WITH YOU!\n\
                            beef: WHAT IS YOUR RESPONSE?!\n"
                        ));
                }
                // only battle player can guess
                BeefMessage::BattleGuess(guess) => {
                    // find battle im in and get ids
                    let Some(mut current_battle) =
                        battles.lock().unwrap().get_current_battle(&client_id)
                    else {
                        client_stream.send_msg("beef: NO BEEF TO GUESS!");
                        continue;
                    };
                    let to_id = current_battle.id.0;
                    if to_id.eq(&client_id) {
                        client_stream.send_msg("beef: CAN'T SECOND-GUESS YOURSELF!");
                        continue;
                    }

                    let out_stream = clients.lock().unwrap().get_stream(&to_id);
                    let guess = String::from_utf8_lossy(&guess).to_string();

                    if current_battle.check_guess(guess.clone()) {
                        clean_current_battle(
                            battles.lock().unwrap(),
                            clients.lock().unwrap(),
                            &client_id,
                            &to_id,
                            &current_battle.id,
                        );
                        out_stream.send_msg("beef: GUESS CORRECT, BEEF SQUASHED!");
                        client_stream.send_msg("beef: GUESS CORRECT, BEEF SQUASHED!");
                    } else {
                        out_stream.send_msg_string(format!("beef: WRONG GUESS {guess}"));
                        client_stream.send_msg("beef: WRONG GUESS!");
                        battles.lock().unwrap().update_or_add_battle(current_battle);
                    }
                }
                BeefMessage::BattleForfeit => {
                    let Some(current_battle) =
                        battles.lock().unwrap().get_current_battle(&client_id)
                    else {
                        client_stream.send_msg("beef: NO BEEF TO FORFEIT!");
                        continue;
                    };
                    let to_id = current_battle.get_opponnent(&client_id);
                    let out_stream = clients.lock().unwrap().get_stream(&to_id);
                    clean_current_battle(
                        battles.lock().unwrap(),
                        clients.lock().unwrap(),
                        &client_id,
                        &to_id,
                        &current_battle.id,
                    );
                    out_stream.send_msg("beef: OPPONENT FORFEITED!");
                    client_stream.send_msg("beef: BEEF FORFEITED!");
                }
                // only battle master can send messages
                BeefMessage::Message(payload) => {
                    // find battle im in and get ids
                    let current_battle = battles
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|&battle| client_id.eq(&battle.id.0))
                        .cloned();
                    if current_battle.is_none() {
                        client_stream.send_msg("beef: CAN'T MSG, NO BEEFS WITH OTHERS");
                        continue;
                    }
                    let to_id = current_battle.unwrap().id.1;
                    let out_stream = clients.lock().unwrap().get_stream(&to_id);
                    let payload = String::from_utf8_lossy(&payload).to_string();
                    out_stream.send_msg_string(format!("{client_id:x}: {payload}"));
                }

                BeefMessage::Disconnect => {
                    cleanup(clients.lock().unwrap(), battles.lock().unwrap(), &client_id);
                    drop(client_stream);
                    break;
                }
                BeefMessage::NotBeef => {
                    client_stream.send_msg("beef: NOT BEEF COMMAND");
                }
            }
        }
    }
    Ok(())
}

// save a reference to the stream to the clients db, where it can be retrieved via clientId
fn check_client(
    stream: Box<GenericStream>,
    clients: &Arc<Mutex<Clients>>,
    battles: &Arc<Mutex<Battles>>,
) -> Option<ClientId> {
    let buf_reader = BufReader::new(stream.get_clone());
    let request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let protocol_identifier = &request.first().cloned().unwrap_or("".to_string());
    // find out type of request, if beef is the only thing sent, continue execution
    // otherwise just serve html once or return 404
    if "GET / HTTP/1.1".eq(protocol_identifier) {
        serve_info_site(&stream, &clients.lock().unwrap(), &battles.lock().unwrap());
        return None;
    } else if !"beef".eq(protocol_identifier) {
        let response = "HTTP/1.1 404 Not Found\r\n\r\n";
        stream.get_clone().write_all(response.as_bytes()).unwrap();
        return None;
    }

    stream.send_msg("beef: PROTOCOL ENGAGED");

    // just generate some unique id, unix stream get "randomized", tcp from address
    let client_id: ClientId = get_hash(&stream.get_unique_string());
    let is_existing = clients.lock().unwrap().contains_key(&client_id);
    if is_existing {
        stream.send_msg("beef: AUTH EXISTING, VALIDATE m<password>");
    } else {
        stream.send_msg("beef: AUTH NEW, SET PASSWORD m<password>:");
    }

    //
    let password = stream.receive_msg().unwrap().get_payload()?;

    if is_existing {
        if clients
            .lock()
            .unwrap()
            .get(&client_id)
            .unwrap()
            .check_password(&password)
        {
            // i just let this silently fail and not consume stream
            stream.send_msg_string(format!("beef: AUTH SUCCESS {client_id:x}"));
            return Some(client_id);
        }
        stream.send_msg_string(format!("beef: AUTH FAILURE {client_id:x}"));
        return None;
    }

    stream.send_msg_string(format!(
        "beef: WELCOME, {client_id:x}!
      ENTER l TO LIST OTHER USERS,
      ENTER d TO DISCONNECT,
      ENTER b<id><word> TO BEEF WITH USER!"
    ));

    // consume new stream to global datastore
    clients
        .lock()
        .unwrap()
        .insert(client_id, Client::new(password, *stream));
    Some(client_id)
}

fn start_new_beef(
    my_id: ClientId,
    to_id: ClientId,
    target: Payload,
    my_stream: &GenericStream,
    clients: MutexGuard<Clients>,
    battles: MutexGuard<Battles>,
) -> Result<Battle, ()> {
    if my_id.eq(&to_id) {
        my_stream.send_msg("beef: CAN'T BEEF WITH YOURSELF");
        return Err(());
    }
    if battles.exists_by_id(&my_id) {
        my_stream.send_msg("beef: CAN'T BEEF, ALREADY BEEFING!");
        return Err(());
    }
    if battles.exists_by_id(&to_id) {
        my_stream.send_msg_string(format!("beef: CAN'T BEEF, USER {to_id:x} BUSY!"));
        return Err(());
    }
    if !clients.contains_key(&to_id) {
        my_stream.send_msg_string(format!("beef: CAN'T BEEF, USER #{to_id:x} IS NOT ONLINE!"));
        return Err(());
    }

    Ok(Battle::new(
        my_id,
        to_id,
        String::from_utf8_lossy(&target).to_string(),
    ))
}

fn clean_current_battle(
    mut battles: MutexGuard<Battles>,
    mut clients: MutexGuard<Clients>,
    my_id: &ClientId,
    to_id: &ClientId,
    battle_id: &BattleId,
) {
    clients.update_battle_status(my_id, to_id, false);
    battles.del_battle(battle_id);
}

fn cleanup(
    mut clients: MutexGuard<Clients>,
    mut battles: MutexGuard<Battles>,
    client_id: &ClientId,
) {
    clients.remove_user(client_id);
    if let Some(other_id) = battles.del_battle_by_client(client_id) {
        clients.remove_battle_status(&other_id);
    }
}
