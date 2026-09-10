use std::{net::ToSocketAddrs, time::{Duration, Instant}};
use base64::{Engine, engine::general_purpose::STANDARD};
use roughenough_client::Client;
use roughenough_protocol::tags::PublicKey;
fn main() {
    for (name, key) in [
        ("roughtime.se:2002", "S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI="),
        ("time.txryan.com:2002", "iBVjxg/1j7y1+kQUTBYdTabxCppesU/07D4PMDJk2WA="),
        ("roughtime.cloudflare.com:2003", "0GD7c3yP8xEc4Zl2zeuN2SlLvDVVocjsPSL8/Rl/7zg="),
    ] {
        let key: [u8; 32] = STANDARD.decode(key).unwrap().try_into().unwrap();
        let Some(address) = name.to_socket_addrs().unwrap().find(|a| a.is_ipv4()) else { continue };
        let client = Client::builder(address).hostname(name).public_key(PublicKey::from(key)).timeout(Duration::from_secs(2)).build();
        let start = Instant::now();
        match client.query() {
            Ok(m) => println!("{name}: verified midpoint={} radius_s={} elapsed_ms={} nonce_match={} version_match={}",m.midpoint(),m.radius(),start.elapsed().as_millis(),m.response().nonc()==m.request().nonc(),m.response().srep().ver()==m.request().ver()),
            Err(e) => println!("{name}: refused {e}; elapsed_ms={}",start.elapsed().as_millis())
        }
    }
}
