//! A scripted MySQL server on loopback: the handshake (optionally upgraded to TLS),
//! `caching_sha2_password` fast authentication, the text-protocol `SET`/`KILL`
//! statements and the prepared-statement protocol with binary result rows. It
//! records what a client sent, so a test can assert on the wire, not on the
//! adapter's own account of it. No database, container or network outside
//! loopback is involved. Fictional fixture material only.
#![allow(dead_code)]
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    sync::Notify,
};

pub const PASSWORD: &str = "fixture-password";
pub const CONNECTION_ID: u32 = 7;
/// A provider detail that must never reach a caller.
pub const PRIVATE_DETAIL: &str = "private-provider-detail fixture-password";

const LONG_PASSWORD: u32 = 0x1;
const LONG_FLAG: u32 = 0x4;
const CONNECT_WITH_DB: u32 = 0x8;
const PROTOCOL_41: u32 = 0x200;
const SSL: u32 = 0x800;
const TRANSACTIONS: u32 = 0x2000;
const SECURE_CONNECTION: u32 = 0x8000;
const MULTI_STATEMENTS: u32 = 0x1_0000;
const MULTI_RESULTS: u32 = 0x2_0000;
const PS_MULTI_RESULTS: u32 = 0x4_0000;
const PLUGIN_AUTH: u32 = 0x8_0000;
const PLUGIN_AUTH_LENENC: u32 = 0x20_0000;
const DEPRECATE_EOF: u32 = 0x100_0000;
const NONCE: [u8; 20] = *b"fixture-nonce-012345";

pub const LONGLONG: u8 = 0x08;
pub const LONG: u8 = 0x03;
pub const DOUBLE: u8 = 0x05;
pub const TIMESTAMP: u8 = 0x07;
pub const DATE: u8 = 0x0a;
pub const TIME: u8 = 0x0b;
pub const DATETIME: u8 = 0x0c;
pub const JSON: u8 = 0xf5;
pub const NEWDECIMAL: u8 = 0xf6;
pub const VAR_STRING: u8 = 0xfd;
pub const UNSIGNED: u16 = 0x20;
pub const BINARY_CHARSET: u16 = 63;
pub const UTF8MB4: u16 = 255;

#[derive(Clone)]
pub struct Col {
    pub name: &'static str,
    pub ty: u8,
    pub flags: u16,
    pub charset: u16,
    pub decimals: u8,
}
pub fn col(name: &'static str, ty: u8, flags: u16, charset: u16, decimals: u8) -> Col {
    Col {
        name,
        ty,
        flags,
        charset,
        decimals,
    }
}

/// One binary-protocol cell: `None` is SQL NULL, otherwise the encoded value.
pub type Cell = Option<Vec<u8>>;
pub fn int8(value: i64) -> Cell {
    Some(value.to_le_bytes().to_vec())
}
pub fn uint8(value: u64) -> Cell {
    Some(value.to_le_bytes().to_vec())
}
pub fn int4(value: i32) -> Cell {
    Some(value.to_le_bytes().to_vec())
}
pub fn double(value: f64) -> Cell {
    Some(value.to_le_bytes().to_vec())
}
pub fn bytes(value: &[u8]) -> Cell {
    let mut out = lenenc(value.len());
    out.extend_from_slice(value);
    Some(out)
}
pub fn text(value: &str) -> Cell {
    bytes(value.as_bytes())
}
pub fn date(year: u16, month: u8, day: u8) -> Cell {
    let mut out = vec![4];
    out.extend(year.to_le_bytes());
    out.extend([month, day]);
    Some(out)
}
pub fn datetime(year: u16, month: u8, day: u8, hms: [u8; 3], micros: u32) -> Cell {
    let mut out = vec![11];
    out.extend(year.to_le_bytes());
    out.extend([month, day]);
    out.extend(hms);
    out.extend(micros.to_le_bytes());
    Some(out)
}
pub fn time(negative: bool, days: u32, hms: [u8; 3], micros: u32) -> Cell {
    let mut out = vec![12, negative as u8];
    out.extend(days.to_le_bytes());
    out.extend(hms);
    out.extend(micros.to_le_bytes());
    Some(out)
}

#[derive(Clone)]
pub struct Script {
    /// Advertise TLS and upgrade with this configuration.
    pub tls: Option<Arc<tokio_rustls::rustls::ServerConfig>>,
    pub columns: Vec<Col>,
    pub rows: Vec<Vec<Cell>>,
    pub params: u16,
    /// `(code, sqlstate)` answered to the first statement preparation.
    pub prepare_error: Option<(u16, &'static str)>,
    /// `(code, sqlstate)` answered to statement execution.
    pub execute_error: Option<(u16, &'static str)>,
    /// `(code, sqlstate)` answered to authentication.
    pub auth_error: Option<(u16, &'static str)>,
    /// Stall execution until a `KILL QUERY` arrives on another session.
    pub stall: Option<Arc<Notify>>,
    /// Ignore the `KILL QUERY` and never answer the stalled execution.
    pub ignore_kill: bool,
}
impl Default for Script {
    fn default() -> Self {
        Self {
            tls: None,
            columns: vec![
                col("answer", LONG, 0, BINARY_CHARSET, 0),
                col("nothing", VAR_STRING, 0, UTF8MB4, 0),
            ],
            rows: vec![vec![int4(42), None]],
            params: 0,
            prepare_error: None,
            execute_error: None,
            auth_error: None,
            stall: None,
            ignore_kill: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Login {
    pub user: String,
    pub database: String,
    pub plugin: String,
    pub password_matches: bool,
    pub tls: bool,
}

/// Everything a client sent, in order of arrival.
#[derive(Debug, Default)]
pub struct Log {
    /// TCP sessions accepted.
    pub sessions: usize,
    /// Handshake responses received (a refused plaintext session sends none).
    pub logins: Vec<Login>,
    /// Every statement text, text-protocol and prepared alike, in order.
    pub statements: Vec<String>,
    pub prepares: Vec<String>,
    /// Bound parameters of each execution.
    pub executions: Vec<Vec<Option<String>>>,
    pub kills: Vec<String>,
    pub quits: usize,
    /// Stalled sessions the client closed after its KILL went unanswered.
    pub abandoned: usize,
}

pub struct Server {
    pub port: u16,
    pub log: Arc<Mutex<Log>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn start(script: Script) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Log::default()));
    let killed = Arc::new(Notify::new());
    let shared = log.clone();
    let task = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            shared.lock().unwrap().sessions += 1;
            let (log, script, killed) = (shared.clone(), script.clone(), killed.clone());
            tokio::spawn(async move { session(Box::new(stream), script, log, killed).await });
        }
    });
    Server { port, log, task }
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

struct Wire {
    io: Box<dyn Io>,
    seq: u8,
}
impl Wire {
    async fn read(&mut self) -> Option<Vec<u8>> {
        let mut header = [0; 4];
        self.io.read_exact(&mut header).await.ok()?;
        let len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
        self.seq = header[3].wrapping_add(1);
        let mut body = vec![0; len];
        self.io.read_exact(&mut body).await.ok()?;
        Some(body)
    }
    async fn write(&mut self, body: &[u8]) {
        let len = (body.len() as u32).to_le_bytes();
        let _ = self.io.write_all(&[len[0], len[1], len[2], self.seq]).await;
        self.seq = self.seq.wrapping_add(1);
        let _ = self.io.write_all(body).await;
        let _ = self.io.flush().await;
    }
    async fn ok(&mut self, header: u8) {
        self.write(&[header, 0, 0, 2, 0, 0, 0]).await;
    }
    async fn error(&mut self, code: u16, state: &str) {
        let mut body = vec![0xff];
        body.extend(code.to_le_bytes());
        body.push(b'#');
        body.extend(state.as_bytes());
        body.extend(PRIVATE_DETAIL.as_bytes());
        self.write(&body).await;
    }
}

fn lenenc(len: usize) -> Vec<u8> {
    if len < 251 {
        vec![len as u8]
    } else {
        let mut out = vec![0xfc];
        out.extend((len as u16).to_le_bytes());
        out
    }
}
fn lenenc_str(value: &str) -> Vec<u8> {
    let mut out = lenenc(value.len());
    out.extend(value.as_bytes());
    out
}
fn read_lenenc(body: &mut &[u8]) -> usize {
    let first = body[0];
    *body = &body[1..];
    match first {
        0xfc => {
            let value = u16::from_le_bytes([body[0], body[1]]) as usize;
            *body = &body[2..];
            value
        }
        0xfd => {
            let value = u32::from_le_bytes([body[0], body[1], body[2], 0]) as usize;
            *body = &body[3..];
            value
        }
        value if value < 251 => value as usize,
        other => panic!("unexpected length prefix {other:#x}"),
    }
}
fn cstr(body: &mut &[u8]) -> String {
    let end = body.iter().position(|b| *b == 0).unwrap();
    let value = String::from_utf8(body[..end].to_vec()).unwrap();
    *body = &body[end + 1..];
    value
}
fn definition(column: &Col, name: &str) -> Vec<u8> {
    let mut out = lenenc_str("def");
    out.extend(lenenc_str("fixture"));
    out.extend(lenenc_str("result"));
    out.extend(lenenc_str(""));
    out.extend(lenenc_str(name));
    out.extend(lenenc_str(column.name));
    out.push(0x0c);
    out.extend(column.charset.to_le_bytes());
    out.extend(1024_u32.to_le_bytes());
    out.push(column.ty);
    out.extend(column.flags.to_le_bytes());
    out.push(column.decimals);
    out.extend([0, 0]);
    out
}
fn scramble(password: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(password);
    let second = Sha256::digest(first);
    let mut salted = Sha256::new();
    salted.update(second);
    salted.update(NONCE);
    let salted = salted.finalize();
    let mut out = [0; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = first[index] ^ salted[index];
    }
    out
}

/// The statement a prepare names: what it returns and how many parameters it takes.
struct Plan {
    columns: Vec<(Col, String)>,
    params: u16,
    limit: Option<usize>,
}
fn plan(script: &Script, query: &str) -> Plan {
    let upper = query.trim_start().to_ascii_uppercase();
    if [
        "INSERT", "UPDATE", "DELETE", "SET", "CALL", "CREATE", "DROP",
    ]
    .iter()
    .any(|verb| upper.starts_with(verb))
    {
        // A statement that returns no result set: MySQL prepares it with no columns.
        return Plan {
            columns: vec![],
            params: 0,
            limit: None,
        };
    }
    let wrapped = upper.starts_with("SELECT * FROM (");
    let limit = wrapped.then(|| {
        query
            .rsplit_once(" LIMIT ")
            .map(|(_, n)| n.trim().parse().unwrap())
            .unwrap()
    });
    Plan {
        columns: script
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let name = if wrapped {
                    format!("c{index}")
                } else {
                    column.name.to_owned()
                };
                (column.clone(), name)
            })
            .collect(),
        params: script.params,
        limit,
    }
}

async fn session(io: Box<dyn Io>, script: Script, log: Arc<Mutex<Log>>, killed: Arc<Notify>) {
    let mut wire = Wire { io, seq: 0 };
    let mut capabilities = LONG_PASSWORD
        | LONG_FLAG
        | CONNECT_WITH_DB
        | PROTOCOL_41
        | TRANSACTIONS
        | SECURE_CONNECTION
        | MULTI_STATEMENTS
        | MULTI_RESULTS
        | PS_MULTI_RESULTS
        | PLUGIN_AUTH
        | DEPRECATE_EOF;
    if script.tls.is_some() {
        capabilities |= SSL;
    }
    let mut greeting = vec![10];
    greeting.extend(b"8.0.36-fixture\0");
    greeting.extend(CONNECTION_ID.to_le_bytes());
    greeting.extend(&NONCE[..8]);
    greeting.push(0);
    greeting.extend((capabilities as u16).to_le_bytes());
    greeting.push(255);
    greeting.extend(2_u16.to_le_bytes());
    greeting.extend(((capabilities >> 16) as u16).to_le_bytes());
    greeting.push(21);
    greeting.extend([0; 10]);
    greeting.extend(&NONCE[8..]);
    greeting.push(0);
    greeting.extend(b"caching_sha2_password\0");
    wire.write(&greeting).await;
    let Some(mut response) = wire.read().await else {
        return;
    };
    let mut tls = false;
    if response.len() == 32 {
        // SSLRequest: the client asks to upgrade before it sends any credential.
        let Some(config) = script.tls.clone() else {
            return;
        };
        let seq = wire.seq;
        let acceptor = tokio_rustls::TlsAcceptor::from(config);
        let Ok(stream) = acceptor.accept(wire.io).await else {
            return;
        };
        wire = Wire {
            io: Box::new(stream),
            seq,
        };
        tls = true;
        let Some(next) = wire.read().await else {
            return;
        };
        response = next;
    }
    let client = u32::from_le_bytes(response[..4].try_into().unwrap());
    let mut body = &response[32..];
    let user = cstr(&mut body);
    let auth_len = if client & PLUGIN_AUTH_LENENC != 0 {
        read_lenenc(&mut body)
    } else {
        let len = body[0] as usize;
        body = &body[1..];
        len
    };
    let auth = body[..auth_len].to_vec();
    body = &body[auth_len..];
    let database = if client & CONNECT_WITH_DB != 0 {
        cstr(&mut body)
    } else {
        String::new()
    };
    let plugin = if client & PLUGIN_AUTH != 0 {
        cstr(&mut body)
    } else {
        String::new()
    };
    let password_matches = auth == scramble(PASSWORD.as_bytes());
    log.lock().unwrap().logins.push(Login {
        user,
        database,
        plugin,
        password_matches,
        tls,
    });
    if let Some((code, state)) = script.auth_error {
        wire.error(code, state).await;
        return;
    }
    if !password_matches {
        wire.error(1045, "28000").await;
        return;
    }
    wire.write(&[0x01, 0x03]).await;
    wire.ok(0x00).await;

    let mut statements: Vec<Plan> = Vec::new();
    while let Some(packet) = wire.read().await {
        let (command, body) = (packet[0], &packet[1..]);
        match command {
            0x01 => {
                log.lock().unwrap().quits += 1;
                return;
            }
            0x03 => {
                let query = String::from_utf8(body.to_vec()).unwrap();
                log.lock().unwrap().statements.push(query.clone());
                if query.starts_with("KILL QUERY ") {
                    log.lock().unwrap().kills.push(query);
                    killed.notify_one();
                    wire.ok(0x00).await;
                } else if query.starts_with("SET ") {
                    wire.ok(0x00).await;
                } else {
                    wire.error(1064, "42000").await;
                }
            }
            0x16 => {
                let query = String::from_utf8(body.to_vec()).unwrap();
                {
                    let mut log = log.lock().unwrap();
                    log.statements.push(query.clone());
                    log.prepares.push(query.clone());
                }
                if let Some((code, state)) = script.prepare_error {
                    wire.error(code, state).await;
                    continue;
                }
                let statement = plan(&script, &query);
                let mut ok = vec![0x00];
                ok.extend((statements.len() as u32 + 1).to_le_bytes());
                ok.extend((statement.columns.len() as u16).to_le_bytes());
                ok.extend(statement.params.to_le_bytes());
                ok.extend([0, 0, 0]);
                wire.write(&ok).await;
                for index in 0..statement.params {
                    let parameter = col("?", VAR_STRING, 0, UTF8MB4, 0);
                    wire.write(&definition(&parameter, &format!("?{index}")))
                        .await;
                }
                for (column, name) in &statement.columns {
                    wire.write(&definition(column, name)).await;
                }
                statements.push(statement);
            }
            0x17 => {
                let id = u32::from_le_bytes(body[..4].try_into().unwrap()) as usize;
                let statement = &statements[id - 1];
                let mut rest = &body[9..];
                let mut values = Vec::new();
                if statement.params > 0 {
                    let bitmap_len = (statement.params as usize).div_ceil(8);
                    let bitmap = rest[..bitmap_len].to_vec();
                    rest = &rest[bitmap_len..];
                    assert_eq!(rest[0], 1, "parameter types are bound");
                    let types = rest[1..1 + 2 * statement.params as usize].to_vec();
                    rest = &rest[1 + 2 * statement.params as usize..];
                    for index in 0..statement.params as usize {
                        if bitmap[index / 8] & (1 << (index % 8)) != 0 {
                            values.push(None);
                            continue;
                        }
                        assert!(
                            matches!(types[2 * index], 0x0f | 0xfc | 0xfd | 0xfe),
                            "parameter {index} is sent as text"
                        );
                        let len = read_lenenc(&mut rest);
                        values.push(Some(String::from_utf8(rest[..len].to_vec()).unwrap()));
                        rest = &rest[len..];
                    }
                }
                log.lock().unwrap().executions.push(values);
                if let Some(stall) = &script.stall {
                    stall.notify_one();
                    killed.notified().await;
                    if script.ignore_kill {
                        let _ = wire.io.read_to_end(&mut Vec::new()).await;
                        log.lock().unwrap().abandoned += 1;
                        return;
                    }
                    wire.error(1317, "70100").await;
                    continue;
                }
                if let Some((code, state)) = script.execute_error {
                    wire.error(code, state).await;
                    continue;
                }
                if statement.columns.is_empty() {
                    wire.ok(0x00).await;
                    continue;
                }
                wire.write(&lenenc(statement.columns.len())).await;
                for (column, name) in &statement.columns {
                    wire.write(&definition(column, name)).await;
                }
                let rows = statement.limit.unwrap_or(usize::MAX).min(script.rows.len());
                for row in &script.rows[..rows] {
                    let mut packet = vec![0x00];
                    let mut bitmap = vec![0; (row.len() + 7 + 2) / 8];
                    let mut cells: Vec<u8> = Vec::new();
                    for (index, cell) in row.iter().enumerate() {
                        match cell {
                            None => bitmap[(index + 2) / 8] |= 1 << ((index + 2) % 8),
                            Some(bytes) => cells.extend(bytes),
                        }
                    }
                    packet.extend(bitmap);
                    packet.extend(cells);
                    wire.write(&packet).await;
                }
                wire.ok(0xfe).await;
            }
            0x19 => {}
            0x0e | 0x1a => wire.ok(0x00).await,
            other => panic!("unexpected command {other:#x}"),
        }
    }
}

/// Text columns in `utf8mb4`, as `information_schema` sends names and flags.
pub fn strings(names: &[&'static str]) -> Vec<Col> {
    names
        .iter()
        .map(|name| col(name, VAR_STRING, 0, UTF8MB4, 0))
        .collect()
}
/// The columns of the catalogue answers, as the server describes them.
pub fn database_columns() -> Vec<Col> {
    strings(&["database_name"])
}
pub fn table_columns() -> Vec<Col> {
    vec![
        col("table_name", VAR_STRING, 0, UTF8MB4, 0),
        col("table_kind", VAR_STRING, 0, UTF8MB4, 0),
        col("row_estimate", LONGLONG, UNSIGNED, BINARY_CHARSET, 0),
    ]
}
pub fn describe_columns() -> Vec<Col> {
    let mut columns = strings(&[
        "column_name",
        "native_type",
        "is_nullable",
        "column_default",
    ]);
    columns.push(col("ordinal_position", LONG, UNSIGNED, BINARY_CHARSET, 0));
    columns.push(col(
        "primary_key_position",
        LONG,
        UNSIGNED,
        BINARY_CHARSET,
        0,
    ));
    columns.extend(strings(&[
        "foreign_key",
        "referenced_schema",
        "referenced_table",
        "referenced_column",
    ]));
    columns
}
pub fn index_columns() -> Vec<Col> {
    let mut columns = strings(&["index_name", "table_name"]);
    columns.push(col("column_position", LONG, UNSIGNED, BINARY_CHARSET, 0));
    columns.extend(strings(&["column_name", "is_unique", "is_primary"]));
    columns
}
