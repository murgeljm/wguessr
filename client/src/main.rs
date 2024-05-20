use std::io::{Read, Write};
use std::net::TcpStream;
use std::{io, thread};

use beef_messages::BeefMessage;

fn main() -> std::io::Result<()> {
    // perform initial handshake, sending 'beef\r\n\r\n'
    let mut write_stream = TcpStream::connect("127.0.0.1:1234")?;
    write_stream.write_all("beef\r\n\r\n".as_ref()).unwrap();

    // pipe all from tcp stream to stdout
    let mut read_stream = write_stream.try_clone().unwrap();
    thread::spawn(move || loop {
        let mut buffer = [0; 16];
        match read_stream.read(&mut buffer) {
            Ok(_) => {
                io::stdout().write_all(&buffer).unwrap();
            }
            Err(_) => {
                break;
            }
        }
    });

    // pipe all from stdin to tcp stream
    loop {
        let mut buffer: Vec<_> = io::stdin()
            .bytes()
            .map(|x| x.unwrap())
            .take_while(|x| *x != 10)
            .collect();

        // parse literal  into two bytes (e.g. a03f -> xa0 x3f) for ClientId
        let first = match buffer.first() {
            None => {
                continue;
            }
            Some(x) => x,
        };
        if b"b"[0].eq(first) && buffer.len() >= 5 {
            let [a, b, x, y] = &buffer[1..5] else {
                continue;
            };
            buffer.splice(1..5, parse_literal_into_byte(&[*a, *b, *x, *y]));
        }

        match send_beef(&mut write_stream, buffer.into()) {
            Ok(BeefMessage::Disconnect) => {
                break;
            }
            Ok(_) => {}
            Err(_) => {
                break;
            }
        }
    }

    Ok(())
}

fn send_beef(stream: &mut TcpStream, msg: BeefMessage) -> std::io::Result<BeefMessage> {
    let msg_raw: Vec<u8> = msg.clone().into();
    stream.try_clone()?.write_all(msg_raw.as_slice())?;
    Ok(msg)
}

pub fn parse_literal_into_byte(literal: &[u8; 4]) -> [u8; 2] {
    let mut result = [0; 2];
    for i in 0..2 {
        let first_in_pair = match literal[i * 2] {
            b @ b'0'..=b'9' => (b - b'0') << 4,
            b @ b'a'..=b'f' => (b - b'a' + 10) << 4,
            b @ b'A'..=b'F' => (b - b'A' + 10) << 4,
            _ => b'0' << 4,
        };

        // bitwise or with second byte
        result[i] = first_in_pair
            | match literal[i * 2 + 1] {
                b @ b'0'..=b'9' => b - b'0',
                b @ b'a'..=b'f' => b - b'a' + 10,
                b @ b'A'..=b'F' => b - b'A' + 10,
                _ => b'0',
            };
    }
    result
}
