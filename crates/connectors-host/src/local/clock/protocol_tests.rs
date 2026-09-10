use super::super::fixture;
use super::*;
const REQUEST: &[u8] = include_bytes!("../../../tests/fixtures/clock/roughtime-se-request.bin");
const REPLY: &[u8] = include_bytes!("../../../tests/fixtures/clock/roughtime-se-response.bin");
fn golden() -> ([u8; 32], [u8; 32]) {
    use base64::Engine;
    let nonce = frame(REQUEST)
        .unwrap()
        .get(b"NONC")
        .unwrap()
        .try_into()
        .unwrap();
    let key = base64::engine::general_purpose::STANDARD
        .decode("S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI=")
        .unwrap()
        .try_into()
        .unwrap();
    (nonce, key)
}
#[test]
fn independent_signed_fixture_verifies_without_a_wall_clock() {
    let (n, k) = golden();
    let s = verify(REPLY, REQUEST, &n, &k).unwrap();
    assert_eq!(s.midpoint, 1_789_076_475);
    assert_eq!(s.radius, 1);
    assert!(verify(REPLY, REQUEST, &[0; 32], &k).is_err());
    assert!(verify(REPLY, REQUEST, &n, &[0; 32]).is_err());
    let mut request = REQUEST.to_vec();
    *request.last_mut().unwrap() ^= 1;
    assert!(verify(REPLY, &request, &n, &k).is_err());
}
#[test]
fn every_truncation_and_single_byte_change_in_known_reply_refuses() {
    let (n, k) = golden();
    for len in 0..REPLY.len() {
        assert!(
            verify(&REPLY[..len], REQUEST, &n, &k).is_err(),
            "length {len}"
        );
    }
    for offset in 0..REPLY.len() {
        let mut b = REPLY.to_vec();
        b[offset] ^= 1;
        assert!(verify(&b, REQUEST, &n, &k).is_err(), "offset {offset}");
    }
    let mut b = REPLY.to_vec();
    b.extend_from_slice(&[0; 4]);
    assert!(verify(&b, REQUEST, &n, &k).is_err());
}
#[test]
fn authentic_unknown_fields_versions_and_both_merkle_directions_work() {
    let k = fixture::root_key();
    let (r, n) = request(&k).unwrap();
    for index in [0, 1] {
        let f = fixture::Fixture {
            extra: true,
            index,
            sibling: Some([42; 32]),
            versions: vec![1, 0x80000001, VERSION, u32::MAX],
            ..Default::default()
        };
        let b = f.reply(&r);
        assert!(verify(&b, &r, &n, &k).is_ok());
    }
}
#[test]
fn signed_semantic_errors_do_not_supply_time() {
    let k = fixture::root_key();
    let (r, n) = request(&k).unwrap();
    let bad = [
        fixture::Fixture {
            version: 1,
            ..Default::default()
        },
        fixture::Fixture {
            versions: vec![],
            ..Default::default()
        },
        fixture::Fixture {
            versions: vec![VERSION, VERSION],
            ..Default::default()
        },
        fixture::Fixture {
            versions: vec![VERSION, 1],
            ..Default::default()
        },
        fixture::Fixture {
            versions: vec![1],
            ..Default::default()
        },
        fixture::Fixture {
            versions: (0..33).collect(),
            ..Default::default()
        },
        fixture::Fixture {
            radius: 0,
            ..Default::default()
        },
        fixture::Fixture {
            mint: u64::MAX,
            ..Default::default()
        },
        fixture::Fixture {
            maxt: 0,
            ..Default::default()
        },
        fixture::Fixture {
            msg_type: 0,
            ..Default::default()
        },
        fixture::Fixture {
            index: 1,
            ..Default::default()
        },
        fixture::Fixture {
            index: 2,
            sibling: Some([42; 32]),
            ..Default::default()
        },
        fixture::Fixture {
            nonce_override: Some([0; 32]),
            ..Default::default()
        },
    ];
    for (i, f) in bad.into_iter().enumerate() {
        assert!(verify(&f.reply(&r), &r, &n, &k).is_err(), "case {i}");
    }
    for midpoint in [1_789_000_000, 1_790_000_000] {
        let f = fixture::Fixture {
            midpoint,
            ..Default::default()
        };
        assert!(verify(&f.reply(&r), &r, &n, &k).is_ok());
    }
}
#[test]
fn duplicate_tags_bad_offsets_nested_lengths_and_signature_substitutions_refuse() {
    let k = fixture::root_key();
    let (r, n) = request(&k).unwrap();
    let b = fixture::Fixture::default().reply(&r);
    for tag in [
        *b"SIG\0", *b"NONC", *b"TYPE", *b"PATH", *b"SREP", *b"CERT", *b"INDX",
    ] {
        let mut fields = fixture::fields(&b[12..]);
        fields.insert(tag, vec![0xff; 4]);
        assert!(verify(&fixture::frame(&fixture::message(fields)), &r, &n, &k).is_err());
    }
    for nested in [*b"CERT", *b"SREP"] {
        let mut fields = fixture::fields(&b[12..]);
        let inner = fields.get_mut(&nested).unwrap();
        inner[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(verify(&fixture::frame(&fixture::message(fields)), &r, &n, &k).is_err());
    }
    let mut duplicate = b.clone();
    duplicate[44..48].copy_from_slice(&b[40..44]);
    assert!(verify(&duplicate, &r, &n, &k).is_err());
    for value in [1, 3, u32::MAX] {
        let mut bad = b.clone();
        bad[16..20].copy_from_slice(&value.to_le_bytes());
        assert!(verify(&bad, &r, &n, &k).is_err());
    }
}
