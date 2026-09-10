use std::{net::{ToSocketAddrs,SocketAddr,UdpSocket}, time::{Duration, Instant}};
use base64::{Engine, engine::general_purpose::STANDARD};
use roughenough_client::{Client, ClientError, transport::ClientTransport};
use roughenough_protocol::tags::PublicKey;
struct Capture(UdpSocket);
impl ClientTransport for Capture {
    fn send(&self,b:&[u8],a:SocketAddr)->Result<usize,ClientError>{std::fs::write("request.bin",b)?;Ok(self.0.send_to(b,a)?)}
    fn recv(&self,b:&mut[u8])->Result<(usize,SocketAddr),ClientError>{let(n,a)=self.0.recv_from(b)?;std::fs::write("response.bin",&b[..n])?;Ok((n,a))}
}
fn main() {
    let name="roughtime.se:2002";
    let key: [u8; 32] = STANDARD.decode("S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI=").unwrap().try_into().unwrap();
    let address = name.to_socket_addrs().unwrap().find(|a| a.is_ipv4()).unwrap();
    let s=UdpSocket::bind("0.0.0.0:0").unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    s.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
    let client = Client::builder(address).hostname(name).public_key(PublicKey::from(key)).transport(Box::new(Capture(s))).build();
    let start = Instant::now();
    let m=client.query().unwrap();
    println!("{name} {address}: verified midpoint={} radius_s={} elapsed_ms={} nonce_match={} version_match={}",m.midpoint(),m.radius(),start.elapsed().as_millis(),m.response().nonc()==m.request().nonc(),m.response().srep().ver()==m.request().ver());
}
