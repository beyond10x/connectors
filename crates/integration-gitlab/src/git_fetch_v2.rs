//! The admitted initial-clone subset of https://git-scm.com/docs/protocol-v2.

use std::collections::BTreeSet;

use async_trait::async_trait;
use service::{EgressByteStream, EgressTransportError};

const MAX_PACKET_BYTES: usize = 65_520;
const MAX_BUFFER_BYTES: usize = 256 * 1024;
const MAX_CAPABILITIES_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    LsRefs { symrefs: bool },
    Fetch,
}

/// Validate completely before egress, then rebuild the provider request from admitted arguments.
pub(super) fn request(
    body: &[u8],
    reference: &str,
    commit: &str,
    maximum_depth: u8,
) -> Option<(Command, Vec<u8>)> {
    if body.len() > MAX_BUFFER_BYTES {
        return None;
    }
    let mut remaining = body;
    let command = text(data(take_packet(&mut remaining)?)?)?;
    let command = match command {
        b"command=ls-refs" => Command::LsRefs { symrefs: false },
        b"command=fetch" => Command::Fetch,
        _ => return None,
    };
    let mut capabilities = BTreeSet::new();
    loop {
        match take_packet(&mut remaining)? {
            Packet::Delimiter => break,
            Packet::Data(line) => {
                let line = text(line)?;
                let name = if line == b"object-format=sha1" {
                    "object-format"
                } else if line.strip_prefix(b"agent=").is_some_and(|agent| {
                    !agent.is_empty()
                        && agent.len() <= 256
                        && agent.iter().all(u8::is_ascii_graphic)
                }) {
                    "agent"
                } else {
                    return None;
                };
                if !capabilities.insert(name) {
                    return None;
                }
            }
            _ => return None,
        }
    }
    let mut arguments = Vec::new();
    loop {
        match take_packet(&mut remaining)? {
            Packet::Flush if remaining.is_empty() => break,
            Packet::Data(line) => arguments.push(text(line)?),
            _ => return None,
        }
    }
    match command {
        Command::LsRefs { .. } => {
            let mut flags = BTreeSet::new();
            let mut prefixes = 0;
            for argument in arguments {
                if matches!(argument, b"symrefs" | b"peel") {
                    if !flags.insert(argument) {
                        return None;
                    }
                } else if argument.strip_prefix(b"ref-prefix ").is_some_and(|prefix| {
                    prefix.len() <= 1_024 && prefix.iter().all(u8::is_ascii_graphic)
                }) {
                    prefixes += 1;
                    if prefixes > 64 {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            let mut upstream = packet(b"command=ls-refs\n");
            upstream.extend_from_slice(b"0001");
            upstream.extend(packet(b"symrefs\n"));
            upstream.extend(packet(b"ref-prefix HEAD\n"));
            upstream.extend(packet(
                format!("ref-prefix refs/heads/{reference}\n").as_bytes(),
            ));
            upstream.extend_from_slice(b"0000");
            Some((
                Command::LsRefs {
                    symrefs: flags.contains(b"symrefs".as_slice()),
                },
                upstream,
            ))
        }
        Command::Fetch => {
            if arguments.last().copied() != Some(b"done".as_slice()) {
                return None;
            }
            let mut seen = BTreeSet::new();
            for argument in &arguments {
                let name = if let Some(want) = argument.strip_prefix(b"want ") {
                    if want != commit.as_bytes() {
                        return None;
                    }
                    b"want".as_slice()
                } else if let Some(depth) = argument.strip_prefix(b"deepen ") {
                    if depth.is_empty() || !depth.iter().all(u8::is_ascii_digit) {
                        return None;
                    }
                    let depth = std::str::from_utf8(depth).ok()?.parse::<u8>().ok()?;
                    if depth == 0 || depth > maximum_depth {
                        return None;
                    }
                    b"deepen".as_slice()
                } else if matches!(
                    *argument,
                    b"thin-pack" | b"ofs-delta" | b"no-progress" | b"done"
                ) {
                    argument
                } else {
                    return None;
                };
                if !seen.insert(name) {
                    return None;
                }
            }
            if ![b"want".as_slice(), b"deepen", b"done"]
                .iter()
                .all(|key| seen.contains(key))
            {
                return None;
            }
            let mut upstream = packet(b"command=fetch\n");
            upstream.extend_from_slice(b"0001");
            for argument in arguments {
                let mut line = argument.to_vec();
                line.push(b'\n');
                upstream.extend(packet(&line));
            }
            upstream.extend_from_slice(b"0000");
            Some((Command::Fetch, upstream))
        }
    }
}

/// Refuse fallback or unsupported object formats; expose only capabilities this proxy implements.
pub(super) async fn capabilities(
    inner: Box<dyn EgressByteStream>,
) -> Result<Vec<u8>, EgressTransportError> {
    let mut reader = PacketReader::new(inner);
    if reader
        .next()
        .await?
        .as_ref()
        .and_then(|packet| packet.line())
        != Some(b"version 2")
    {
        return Err(EgressTransportError::Refused);
    }
    let mut seen = BTreeSet::new();
    let mut shallow = false;
    let mut bytes = 0;
    loop {
        match reader.next().await? {
            Some(OwnedPacket::Flush) => break,
            Some(OwnedPacket::Data(payload)) => {
                bytes += payload.len();
                if bytes > MAX_CAPABILITIES_BYTES {
                    return Err(EgressTransportError::ResponseTooLarge);
                }
                let line = text(&payload).ok_or(EgressTransportError::Refused)?;
                let (key, value) = line.split_once_byte(b'=');
                if key.is_empty()
                    || value.is_some_and(<[u8]>::is_empty)
                    || !key
                        .iter()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(byte))
                    || !seen.insert(key.to_vec())
                {
                    return Err(EgressTransportError::Refused);
                }
                if key == b"fetch" {
                    shallow = value.is_some_and(|value| {
                        value
                            .split(|byte| *byte == b' ')
                            .any(|value| value == b"shallow")
                    });
                }
                if key == b"object-format" && value != Some(b"sha1") {
                    return Err(EgressTransportError::Refused);
                }
            }
            _ => return Err(EgressTransportError::Refused),
        }
    }
    reader.finish().await?;
    if !shallow || !seen.contains(b"ls-refs".as_slice()) {
        return Err(EgressTransportError::Refused);
    }
    let mut result = Vec::new();
    for line in [
        "version 2\n",
        "ls-refs\n",
        "fetch=shallow\n",
        "object-format=sha1\n",
        concat!("agent=connectors/", env!("CARGO_PKG_VERSION"), "\n"),
    ] {
        result.extend(packet(line.as_bytes()));
    }
    result.extend_from_slice(b"0000");
    Ok(result)
}

/// Retain only two rows even if an upstream ignores our optimization prefixes.
pub(super) async fn references(
    inner: Box<dyn EgressByteStream>,
    reference: &str,
    commit: &str,
    symrefs: bool,
) -> Result<Vec<u8>, EgressTransportError> {
    let mut reader = PacketReader::new(inner);
    let expected_ref = format!("refs/heads/{reference}");
    let expected_symref = format!("symref-target:{expected_ref}");
    let mut saw_head = false;
    let mut saw_branch = false;
    loop {
        let payload = match reader.next().await? {
            Some(OwnedPacket::Flush) => break,
            Some(OwnedPacket::Data(payload)) => payload,
            _ => return Err(EgressTransportError::Refused),
        };
        let line = text(&payload).ok_or(EgressTransportError::Refused)?;
        let mut columns = line.split(|byte| *byte == b' ');
        let object = columns
            .next()
            .filter(|object| is_oid(object))
            .ok_or(EgressTransportError::Refused)?;
        let name = columns
            .next()
            .filter(|name| !name.is_empty())
            .ok_or(EgressTransportError::Refused)?;
        let admitted = name == b"HEAD" || name == expected_ref.as_bytes();
        let mut attributes = BTreeSet::new();
        let mut head_symref = false;
        for attribute in columns {
            let (kind, value) = attribute.split_once_byte(b':');
            if !attributes.insert(kind)
                || !match (kind, value) {
                    (b"symref-target", Some(target)) => {
                        head_symref = attribute == expected_symref.as_bytes();
                        !target.is_empty() && (!admitted || name == b"HEAD" && head_symref)
                    }
                    (b"peeled", Some(oid)) => !admitted && is_oid(oid),
                    _ => false,
                }
            {
                return Err(EgressTransportError::Refused);
            }
        }
        if admitted {
            if object != commit.as_bytes() {
                return Err(EgressTransportError::Refused);
            }
            let seen = if name == b"HEAD" {
                if !head_symref {
                    return Err(EgressTransportError::Refused);
                }
                &mut saw_head
            } else {
                &mut saw_branch
            };
            if std::mem::replace(seen, true) {
                return Err(EgressTransportError::Refused);
            }
        }
    }
    reader.finish().await?;
    if !saw_head || !saw_branch {
        return Err(EgressTransportError::Refused);
    }
    let head = if symrefs {
        format!("{commit} HEAD {expected_symref}\n")
    } else {
        format!("{commit} HEAD\n")
    };
    let mut result = packet(head.as_bytes());
    result.extend(packet(format!("{commit} {expected_ref}\n").as_bytes()));
    result.extend_from_slice(b"0000");
    Ok(result)
}

pub(super) struct OneChunk(pub(super) Option<Vec<u8>>);

#[async_trait]
impl EgressByteStream for OneChunk {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        Ok(self.0.take())
    }
}

/// Validate section grammar without accumulating or interpreting the packfile itself.
pub(super) struct PackStream {
    reader: PacketReader,
    state: PackState,
    saw_pack_data: bool,
}

#[derive(Clone, Copy)]
enum PackState {
    Start,
    Shallow,
    PackHeader,
    Pack,
    End,
}

impl PackStream {
    pub(super) fn new(inner: Box<dyn EgressByteStream>) -> Self {
        Self {
            reader: PacketReader::new(inner),
            state: PackState::Start,
            saw_pack_data: false,
        }
    }
}

#[async_trait]
impl EgressByteStream for PackStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        if matches!(self.state, PackState::End) {
            return Ok(None);
        }
        let next = self
            .reader
            .next()
            .await?
            .ok_or(EgressTransportError::Refused)?;
        match (&self.state, &next) {
            (PackState::Start, _) if next.line() == Some(b"shallow-info") => {
                self.state = PackState::Shallow
            }
            (PackState::Start | PackState::PackHeader, _) if next.line() == Some(b"packfile") => {
                self.state = PackState::Pack
            }
            (PackState::Shallow, OwnedPacket::Delimiter) => self.state = PackState::PackHeader,
            (PackState::Shallow, _)
                if next
                    .line()
                    .and_then(|line| line.strip_prefix(b"shallow "))
                    .is_some_and(is_oid) => {}
            (PackState::Pack, OwnedPacket::Data(payload)) => match payload.first() {
                Some(1) if payload.len() > 1 => self.saw_pack_data = true,
                Some(2) => {}
                _ => return Err(EgressTransportError::Refused),
            },
            (PackState::Pack, OwnedPacket::Flush) if self.saw_pack_data => {
                // Do not forward the terminal flush until EOF rules out trailing commands/URLs.
                self.reader.finish().await?;
                self.state = PackState::End;
            }
            _ => return Err(EgressTransportError::Refused),
        }
        Ok(Some(next.encode()))
    }
}

fn is_oid(value: &[u8]) -> bool {
    value.len() == 40
        && value
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

fn text(value: &[u8]) -> Option<&[u8]> {
    let value = value.strip_suffix(b"\n").unwrap_or(value);
    (!value.is_empty() && value.iter().all(|byte| matches!(byte, b' '..=b'~'))).then_some(value)
}

fn packet(payload: &[u8]) -> Vec<u8> {
    let mut result = format!("{:04x}", payload.len() + 4).into_bytes();
    result.extend_from_slice(payload);
    result
}

enum Packet<'a> {
    Flush,
    Delimiter,
    ResponseEnd,
    Data(&'a [u8]),
}
enum OwnedPacket {
    Flush,
    Delimiter,
    ResponseEnd,
    Data(Vec<u8>),
}

impl OwnedPacket {
    fn line(&self) -> Option<&[u8]> {
        match self {
            Self::Data(data) => text(data),
            _ => None,
        }
    }
    fn encode(self) -> Vec<u8> {
        match self {
            Self::Flush => b"0000".to_vec(),
            Self::Delimiter => b"0001".to_vec(),
            Self::ResponseEnd => b"0002".to_vec(),
            Self::Data(data) => packet(&data),
        }
    }
}

fn data(packet: Packet<'_>) -> Option<&[u8]> {
    match packet {
        Packet::Data(data) => Some(data),
        _ => None,
    }
}

fn packet_length(header: &[u8]) -> Option<usize> {
    if header.len() != 4 || !header.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let length = usize::from_str_radix(std::str::from_utf8(header).ok()?, 16).ok()?;
    (length <= MAX_PACKET_BYTES && length != 3).then_some(length)
}

fn take_packet<'a>(remaining: &mut &'a [u8]) -> Option<Packet<'a>> {
    let length = packet_length(remaining.get(..4)?)?;
    let packet = match length {
        0 => Packet::Flush,
        1 => Packet::Delimiter,
        2 => Packet::ResponseEnd,
        _ => Packet::Data(remaining.get(4..length)?),
    };
    *remaining = &remaining[length.max(4)..];
    Some(packet)
}

struct PacketReader {
    inner: Box<dyn EgressByteStream>,
    buffer: Vec<u8>,
    offset: usize,
}

impl PacketReader {
    fn new(inner: Box<dyn EgressByteStream>) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
            offset: 0,
        }
    }

    async fn next(&mut self) -> Result<Option<OwnedPacket>, EgressTransportError> {
        loop {
            if let Some(header) = self.buffer[self.offset..].get(..4) {
                let length = packet_length(header)
                    .ok_or(EgressTransportError::Refused)?
                    .max(4);
                if self.buffer.len() - self.offset >= length {
                    let mut remaining = &self.buffer[self.offset..];
                    let packet =
                        match take_packet(&mut remaining).ok_or(EgressTransportError::Refused)? {
                            Packet::Flush => OwnedPacket::Flush,
                            Packet::Delimiter => OwnedPacket::Delimiter,
                            Packet::ResponseEnd => OwnedPacket::ResponseEnd,
                            Packet::Data(payload) => OwnedPacket::Data(payload.to_vec()),
                        };
                    self.offset += length;
                    return Ok(Some(packet));
                }
            }
            self.buffer.drain(..self.offset);
            self.offset = 0;
            match self.inner.next_chunk().await? {
                Some(chunk)
                    if self
                        .buffer
                        .len()
                        .checked_add(chunk.len())
                        .is_some_and(|size| size <= MAX_BUFFER_BYTES) =>
                {
                    self.buffer.extend(chunk)
                }
                Some(_) => return Err(EgressTransportError::ResponseTooLarge),
                None if self.buffer.is_empty() => return Ok(None),
                None => return Err(EgressTransportError::Refused),
            }
        }
    }

    async fn finish(&mut self) -> Result<(), EgressTransportError> {
        match self.next().await? {
            None => Ok(()),
            Some(OwnedPacket::ResponseEnd) if self.next().await?.is_none() => Ok(()),
            _ => Err(EgressTransportError::Refused),
        }
    }
}

trait SplitByte {
    fn split_once_byte(&self, separator: u8) -> (&[u8], Option<&[u8]>);
}
impl SplitByte for [u8] {
    fn split_once_byte(&self, separator: u8) -> (&[u8], Option<&[u8]>) {
        self.iter()
            .position(|byte| *byte == separator)
            .map_or((self, None), |at| (&self[..at], Some(&self[at + 1..])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct Chunks(VecDeque<Vec<u8>>);
    #[async_trait]
    impl EgressByteStream for Chunks {
        async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
            Ok(self.0.pop_front())
        }
    }

    fn stream(bytes: &[u8], chunk_size: usize) -> Box<dyn EgressByteStream> {
        Box::new(Chunks(
            bytes.chunks(chunk_size).map(<[u8]>::to_vec).collect(),
        ))
    }

    fn lines(lines: &[&str]) -> Vec<u8> {
        let mut result = Vec::new();
        for line in lines {
            result.extend(packet(format!("{line}\n").as_bytes()));
        }
        result.extend_from_slice(b"0000");
        result
    }

    fn command(name: &str, capabilities: &[&str], arguments: &[&str]) -> Vec<u8> {
        let mut result = packet(format!("command={name}\n").as_bytes());
        for capability in capabilities {
            result.extend(packet(format!("{capability}\n").as_bytes()));
        }
        result.extend_from_slice(b"0001");
        result.extend(lines(arguments));
        result
    }

    #[test]
    fn commands_are_closed_and_prefixes_cannot_expand_upstream_discovery() {
        let commit = "a".repeat(40);
        let input = command(
            "ls-refs",
            &["agent=git/test", "object-format=sha1"],
            &["symrefs", "peel", "ref-prefix refs/", "ref-prefix HEAD"],
        );
        let (kind, upstream) = request(&input, "trunk", &commit, 50).unwrap();
        assert_eq!(kind, Command::LsRefs { symrefs: true });
        assert_eq!(
            upstream,
            command(
                "ls-refs",
                &[],
                &["symrefs", "ref-prefix HEAD", "ref-prefix refs/heads/trunk"]
            )
        );
        let want = format!("want {commit}");
        assert!(request(
            &command(
                "fetch",
                &[],
                &[
                    "thin-pack",
                    "ofs-delta",
                    "no-progress",
                    &want,
                    "deepen 50",
                    "done"
                ]
            ),
            "trunk",
            &commit,
            50
        )
        .is_some());
        for argument in [
            "deepen 0",
            "deepen 51",
            "deepen +1",
            "deepen 256",
            "deepen-relative",
            "deepen-since 100",
            "deepen-not refs/heads/private",
            "filter blob:none",
            "want-ref refs/heads/trunk",
            "include-tag",
            "sideband-all",
            "packfile-uris https",
            "have aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "shallow aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "want bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "unknown",
        ] {
            let body = command("fetch", &[], &[&want, "deepen 50", argument, "done"]);
            assert!(request(&body, "trunk", &commit, 50).is_none(), "{argument}");
        }
        for capabilities in [
            vec!["server-option=foo"],
            vec!["object-format=sha256"],
            vec!["agent=bad agent"],
            vec!["agent=a", "agent=b"],
            vec!["object-format=sha1", "object-format=sha1"],
        ] {
            assert!(request(
                &command("fetch", &capabilities, &[&want, "deepen 50", "done"]),
                "trunk",
                &commit,
                50
            )
            .is_none());
        }
        for args in [
            vec![&*want, "done"],
            vec![&*want, "deepen 50"],
            vec![&*want, &*want, "deepen 50", "done"],
            vec![&*want, "deepen 50", "done", "done"],
        ] {
            assert!(request(&command("fetch", &[], &args), "trunk", &commit, 50).is_none());
        }
        for name in ["push", "ls-refs\ncommand=fetch", "object-info"] {
            assert!(request(&command(name, &[], &[]), "trunk", &commit, 50).is_none());
        }
    }

    #[test]
    fn request_framing_refuses_truncation_ambiguity_and_oversize() {
        let valid = command("ls-refs", &[], &["symrefs"]);
        for size in 0..valid.len() {
            assert!(
                request(&valid[..size], "trunk", &"a".repeat(40), 50).is_none(),
                "{size}"
            );
        }
        for suffix in [
            b"0000".as_slice(),
            b"0001",
            b"0002",
            b"0003",
            b"0004",
            b"zzzz",
            &valid,
        ] {
            let mut body = valid.clone();
            body.extend_from_slice(suffix);
            assert!(request(&body, "trunk", &"a".repeat(40), 50).is_none());
        }
        let mut body = valid.clone();
        body[..4].copy_from_slice(b"+014");
        assert!(request(&body, "trunk", &"a".repeat(40), 50).is_none());
        assert!(request(
            &vec![b'0'; MAX_BUFFER_BYTES + 1],
            "trunk",
            &"a".repeat(40),
            50
        )
        .is_none());
        assert!(request(
            &command("ls-refs", &[], &["symrefs", "symrefs"]),
            "trunk",
            &"a".repeat(40),
            50
        )
        .is_none());
    }

    #[tokio::test]
    async fn capabilities_require_v2_shallow_sha1_and_strip_expanding_features() {
        let upstream = lines(&[
            "version 2",
            "agent=git/test",
            "ls-refs=unborn",
            "fetch=shallow wait-for-done filter ref-in-want sideband-all",
            "server-option",
            "object-format=sha1",
            "object-info",
        ]);
        for chunk_size in [1, 3, 4, 11, upstream.len()] {
            let output = capabilities(stream(&upstream, chunk_size)).await.unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(output.contains("fetch=shallow\n"));
            for hidden in [
                "unborn",
                "wait-for-done",
                "filter",
                "ref-in-want",
                "sideband-all",
                "object-info",
                "server-option",
                "agent=git/test",
            ] {
                assert!(!output.contains(hidden), "{hidden}");
            }
        }
        for bad in [
            lines(&["version 1", "ls-refs", "fetch=shallow"]),
            lines(&["version 2", "fetch=shallow"]),
            lines(&["version 2", "ls-refs", "fetch=filter"]),
            lines(&[
                "version 2",
                "ls-refs",
                "fetch=shallow",
                "object-format=sha256",
            ]),
            lines(&["version 2", "ls-refs", "ls-refs", "fetch=shallow"]),
        ] {
            assert!(capabilities(stream(&bad, 1)).await.is_err());
        }
        let large = lines(&[
            "version 2",
            "ls-refs",
            "fetch=shallow",
            &format!("large={}", "x".repeat(MAX_CAPABILITIES_BYTES)),
        ]);
        assert!(matches!(
            capabilities(stream(&large, 4096)).await,
            Err(EgressTransportError::ResponseTooLarge)
        ));
    }

    #[tokio::test]
    async fn refs_filter_prefix_collisions_and_verify_full_response_before_emission() {
        let commit = "a".repeat(40);
        let head = format!("{commit} HEAD symref-target:refs/heads/trunk");
        let branch = format!("{commit} refs/heads/trunk");
        let private = format!("{} refs/heads/trunk-private", "b".repeat(40));
        let upstream = lines(&[&head, &branch, &private]);
        for chunk_size in [1, 3, 7, upstream.len()] {
            assert_eq!(
                references(stream(&upstream, chunk_size), "trunk", &commit, true)
                    .await
                    .unwrap(),
                lines(&[&head, &branch])
            );
        }
        assert_eq!(
            references(stream(&upstream, 7), "trunk", &commit, false)
                .await
                .unwrap(),
            lines(&[&format!("{commit} HEAD"), &branch])
        );
        for bad in [
            lines(&[&head]),
            lines(&[&head, &branch, &branch]),
            lines(&[
                &format!("{commit} HEAD symref-target:refs/heads/private"),
                &branch,
            ]),
            lines(&[&head, &format!("{} refs/heads/trunk", "b".repeat(40))]),
            lines(&[&head, &format!("{branch} peeled:{commit}")]),
        ] {
            assert!(references(stream(&bad, 1), "trunk", &commit, true)
                .await
                .is_err());
        }
        for size in 0..upstream.len() {
            assert!(
                references(stream(&upstream[..size], 7), "trunk", &commit, true)
                    .await
                    .is_err()
            );
        }
        let mut extra = upstream;
        extra.extend(packet(b"injected\n"));
        assert!(references(stream(&extra, 1), "trunk", &commit, true)
            .await
            .is_err());
    }

    async fn read_pack(bytes: &[u8], size: usize) -> Result<Vec<u8>, EgressTransportError> {
        let mut stream = PackStream::new(stream(bytes, size));
        let mut output = Vec::new();
        while let Some(chunk) = stream.next_chunk().await? {
            output.extend(chunk);
        }
        Ok(output)
    }

    #[tokio::test]
    async fn pack_is_streamed_but_final_framing_and_sections_are_enforced() {
        let mut valid = packet(b"shallow-info\n");
        valid.extend(packet(format!("shallow {}\n", "a".repeat(40)).as_bytes()));
        valid.extend_from_slice(b"0001");
        valid.extend(packet(b"packfile\n"));
        valid.extend(packet(b"\x02progress\n"));
        valid.extend(packet(b"\x01PACKdata"));
        valid.extend_from_slice(b"0000");
        for size in [1, 4, 7, valid.len()] {
            assert_eq!(read_pack(&valid, size).await.unwrap(), valid);
        }
        for size in 0..valid.len() {
            assert!(read_pack(&valid[..size], 3).await.is_err());
        }
        for line in [
            "acknowledgments",
            "wanted-refs",
            "packfile-uris",
            "shallow-info\nunshallow deadbeef",
        ] {
            assert!(read_pack(&lines(&[line]), 1).await.is_err());
        }
        for band in [0, 3, 4] {
            let mut body = packet(b"packfile\n");
            body.extend(packet(&[band, 9]));
            body.extend_from_slice(b"0000");
            assert!(read_pack(&body, 1).await.is_err());
        }
        let mut extra = valid;
        extra.extend(packet(b"packfile-uris\n"));
        assert!(read_pack(&extra, 1).await.is_err());
    }
}
