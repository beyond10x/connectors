//! Operator-driven, non-stable characterization of an exact non-loopback `sip.dial` route.

use std::collections::BTreeSet;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use connector_resolve::document::Document;
use domain::voice::{AudioFrame, TerminationReason};
use domain::{AdmittedOperation, Capability, ConnectionAuthority, DriverId, InitiationPolicy};
use protocol::sip::{SipDialInput, SIP_DIAL_OPERATION, SIP_DIAL_TOOL_REF};
use server::{
    admit_sip_dial, CredentialSet, SipDeploymentRoute, SipDialRouteTable, SipNetworkMode,
    SipSignalingTransport, SocketAperture,
};
use service::{plan_operation, PlanningEnvironment};
use tokio_util::sync::CancellationToken;

fn required(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} is required"))
}

fn address(name: &str) -> SocketAddr {
    required(name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be an IP socket address"))
}

fn ip(name: &str) -> IpAddr {
    required(name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be an IP address"))
}

fn port(name: &str) -> u16 {
    required(name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be a port"))
}

#[tokio::main]
async fn main() {
    let connection = "asterisk-dev-characterization";
    let alias = "asterisk-dev";
    let document = Document::parse(include_str!("../../../catalog/asterisk.catalog.json"))
        .expect("canonical Asterisk catalog parses");
    let operation = document
        .operation(SIP_DIAL_OPERATION)
        .expect("sip.dial is published");
    let plan = plan_operation(
        "asterisk",
        operation,
        AdmittedOperation::from_grant_decision(
            "asterisk",
            SIP_DIAL_OPERATION,
            "development",
            "operator",
            "characterization-grant",
            ConnectionAuthority::new(connection, InitiationPolicy::b10x_only()).unwrap(),
        ),
        &PlanningEnvironment {
            available_drivers: BTreeSet::from([DriverId::SipV1]),
            available_route_adapters: BTreeSet::new(),
            capabilities: BTreeSet::from([Capability::PrivateNetwork]),
            permission_subjects: vec![format!("connection-target:{alias}")],
        },
    )
    .expect("sip.dial plans");

    let signal_target = address("B10X_SIP_SIGNAL_TARGET");
    let signal_peer = ip("B10X_SIP_SIGNAL_PEER_IP");
    let signal_peer_port = port("B10X_SIP_SIGNAL_PEER_PORT");
    let media_peer = ip("B10X_SIP_MEDIA_PEER_IP");
    let media_start = port("B10X_SIP_MEDIA_PORT_START");
    let media_end = port("B10X_SIP_MEDIA_PORT_END");
    let media_bind = ip("B10X_SIP_MEDIA_BIND");
    let media_advertised = ip("B10X_SIP_MEDIA_ADVERTISED");
    let signal_bind = address("B10X_SIP_SIGNAL_BIND");
    let routes = SipDialRouteTable::new(
        connection,
        [(
            alias.to_owned(),
            SipDeploymentRoute {
                connection: connection.to_owned(),
                signaling_bind: signal_bind,
                sent_by: required("B10X_SIP_SENT_BY"),
                target: signal_target,
                signaling_transport: SipSignalingTransport::Tcp,
                to_uri: required("B10X_SIP_TO_URI"),
                from_uri: required("B10X_SIP_FROM_URI"),
                media_advertised,
                media_bind,
                signaling_apertures: vec![
                    SocketAperture::new(signal_bind.ip(), 1..=u16::MAX).unwrap(),
                    SocketAperture::new(
                        signal_target.ip(),
                        signal_target.port()..=signal_target.port(),
                    )
                    .unwrap(),
                    SocketAperture::new(signal_peer, signal_peer_port..=signal_peer_port).unwrap(),
                ],
                media_apertures: vec![
                    SocketAperture::new(media_bind, 1..=u16::MAX).unwrap(),
                    SocketAperture::new(media_advertised, 1..=u16::MAX).unwrap(),
                    SocketAperture::new(media_peer, media_start..=media_end).unwrap(),
                ],
                dial_timeout: Duration::from_secs(10),
                network_mode: SipNetworkMode::OperatorAuthorizedDevelopment,
            },
        )],
    )
    .unwrap();
    let admitted = admit_sip_dial(
        &plan,
        &SipDialInput {
            target: alias.to_owned(),
        },
        &routes,
    )
    .expect("exact characterization route is admitted");
    let session = driver_sip::establish_outbound(
        &admitted,
        &CredentialSet::default(),
        CancellationToken::new(),
    )
    .await
    .expect("Asterisk answered sip.dial");
    let descriptor = session.descriptor();
    let expected = 1_234_i16;
    let bytes = [expected]
        .repeat(160)
        .into_iter()
        .flat_map(i16::to_le_bytes)
        .collect::<Vec<_>>();
    for sequence in 1..=20 {
        session
            .write_output(AudioFrame::new(sequence, bytes.clone(), &descriptor.media).unwrap())
            .await
            .expect("RTP write succeeds");
    }
    let mut media_echo = false;
    for _ in 0..40 {
        let Ok(Ok(Some(frame))) =
            tokio::time::timeout(Duration::from_millis(250), session.read_input()).await
        else {
            continue;
        };
        let close_samples = frame
            .bytes
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .filter(|actual| (i32::from(*actual) - i32::from(expected)).abs() <= 128)
            .count();
        if close_samples >= 150 {
            media_echo = true;
            break;
        }
    }
    assert!(media_echo, "Asterisk did not echo the admitted RTP profile");
    println!(
        "{}",
        serde_json::json!({
            "operation": SIP_DIAL_TOOL_REF,
            "target": alias,
            "state": "sip_established",
            "call": descriptor.call.as_str(),
            "session": descriptor.session.as_str(),
            "channel": descriptor.channel.as_str(),
            "media_echo": media_echo
        })
    );
    session
        .terminate(TerminationReason::Completed)
        .await
        .expect("characterization call terminates");
}
