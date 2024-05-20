use std::io::Write;

use crate::battle::{BattleDatabase, Battles};
use crate::client::{ClientDatabase, Clients};
use crate::generic_stream::GenericStream;

// I would usually use a library to handle this, but format!() is surprisingly capable.
pub fn serve_info_site(stream: &GenericStream, clients: &Clients, battles: &Battles) {
    let status_line = "HTTP/1.1 200 OK";
    let contents = format!(
        "<!DOCTYPE html>
        <html lang=\"en\">
        <head>
            <meta charset=\"utf-8\">
            <title>wguessr</title>
        </head>
        <body style=\"background: #333030; color: #dddddd\">
            <pre style=\"color: #ff0000\">

     █     █░ ▄████  █    ██ ▓█████   ██████   ██████  ██▀███
    ▓█░ █ ░█░██▒ ▀█▒ ██  ▓██▒▓█   ▀ ▒██    ▒ ▒██    ▒ ▓██ ▒ ██▒
    ▒█░ █ ░█▒██░▄▄▄░▓██  ▒██░▒███   ░ ▓██▄   ░ ▓██▄   ▓██ ░▄█ ▒
    ░█░ █ ░█░▓█  ██▓▓▓█  ░██░▒▓█  ▄   ▒   ██▒  ▒   ██▒▒██▀▀█▄
    ░░██▒██▓░▒▓███▀▒▒▒█████▓ ░▒████▒▒██████▒▒▒██████▒▒░██▓ ▒██▒
    ░ ▓░▒ ▒  ░▒   ▒ ░▒▓▒ ▒ ▒ ░░ ▒░ ░▒ ▒▓▒ ▒ ░▒ ▒▓▒ ▒ ░░ ▒▓ ░▒▓░
      ▒ ░ ░   ░   ░ ░░▒░ ░ ░  ░ ░  ░░ ░▒  ░ ░░ ░▒  ░ ░  ░▒ ░ ▒░
      ░   ░ ░ ░   ░  ░░░ ░ ░    ░   ░  ░  ░  ░  ░  ░    ░░   ░
        ░         ░    ░        ░  ░      ░        ░     ░

            </pre>
            <div style=\"display: flex; margin-left: 1em;\">
                <div style=\"flex-grow: 1;\">
                    <strong>Clients online:</strong><br>
                    <ul style=\"padding-left: 1.2em;\">{}</ul>
                </div>
                <div style=\"flex-grow: 3;\">
                    <strong>Battles happening:</strong><br>
                    {}
                </div>
            </div>
        </body>
        </html>
    ",
        clients.to_html_string(),
        battles.to_html_string()
    );
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.get_clone().write_all(response.as_bytes()).unwrap();
}
