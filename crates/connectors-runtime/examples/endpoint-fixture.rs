//! Isolated cluster acceptance fixture for declared Loki reads and Asterisk ARI HTTP/WebSocket.
//! This is a test service, with its credentials supplied only by the fixture pod environment.

use base64::Engine as _;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let password =
        std::env::var("FIXTURE_PASSWORD").expect("fixture password environment required");
    let authorization = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("fixture-user:{password}"))
    );
    let mut threads = Vec::new();
    for port in [3100, 8088, 9000] {
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        let authorization = authorization.clone();
        threads.push(std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let authorization = authorization.clone();
                std::thread::spawn(move || {
                    let _ = serve(stream, &authorization);
                });
            }
        }));
    }
    for thread in threads {
        let _ = thread.join();
    }
    Ok(())
}

fn serve(mut stream: TcpStream, authorization: &str) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut buffer = [0; 16 * 1024];
    let count = loop {
        let count = stream.peek(&mut buffer)?;
        if count == 0 || count == buffer.len() {
            return Ok(());
        }
        if buffer[..count].windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            break count;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let request = String::from_utf8_lossy(&buffer[..count]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let authenticated = request
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .any(|(name, value)| {
            name.eq_ignore_ascii_case("authorization") && value.trim() == authorization
        });
    if path.starts_with("/ari/") && !authenticated {
        return respond(
            &mut stream,
            "401 Unauthorized",
            "{\"message\":\"fixture authentication required\"}",
        );
    }
    if path.starts_with("/ari/events?") && authenticated {
        let mut socket = tungstenite::accept(stream).map_err(std::io::Error::other)?;
        let event = serde_json::json!({"type":"ApplicationRegistered","application":"fixture",
            "timestamp":"2026-09-07T12:00:00Z"})
        .to_string();
        socket
            .send(tungstenite::Message::Text(event.into()))
            .map_err(std::io::Error::other)?;
        loop {
            match socket.read() {
                Ok(tungstenite::Message::Close(_)) | Err(_) => break,
                Ok(tungstenite::Message::Ping(bytes)) => {
                    let _ = socket.send(tungstenite::Message::Pong(bytes));
                }
                _ => {}
            }
        }
        return Ok(());
    }
    let body = if path == "/ari/applications" {
        "[]"
    } else if path.starts_with("/loki/api/v1/query_range") {
        "{\"status\":\"success\",\"data\":{\"resultType\":\"streams\",\"result\":[{\"stream\":{\"job\":\"fixture\"},\"values\":[[\"1788804000000000000\",\"fixture log\"]]}]}}"
    } else if path.starts_with("/loki/api/v1/labels") {
        "{\"status\":\"success\",\"data\":[\"fixture\"]}"
    } else if path == "/ready" {
        "{}"
    } else {
        "{\"fixture\":true}"
    };
    // Consume the request before responding so the peer receives an ordinary orderly close.
    let _ = stream.read(&mut buffer[..count]);
    respond(&mut stream, "200 OK", body)
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}
