//! **Raw SIP**: establish a call at the edge and terminate it there.
//!
//! # Why this exists
//!
//! `sip.dial` could not be configured, admitted or launched without an application channel — not
//! because SIP needed one, but because every layer only knew the composed shape. The driver has
//! always been able to do this alone: [`driver_sip::establish_outbound`] takes an admitted plan and
//! returns a [`TelephonySession`], which *is* the neutral media session contract. SIP produces one;
//! the application-channel binding produces one; neither is a stage of the other.
//!
//! This launcher is the missing arm — the one that stops after establishing, because there is
//! nowhere the call is being carried onward to.
//!
//! # What it deliberately does not do yet
//!
//! **Nothing consumes the media.** The call connects, SDP is negotiated and RTP flows, and the
//! frames arrive at a session nobody reads. That is honest for this slice and it is not the end
//! state: hearing a call on the machine that placed it needs a duplex binding between
//! [`TelephonySession::read_input`] / [`TelephonySession::write_output`] and the local audio device,
//! and **no capture path exists anywhere in the tree** — `driver-audio` is text-to-speech, output
//! only, through `pw-play`. That binding is its own piece of work, and it is named here rather than
//! faked.

use std::sync::Arc;

use async_trait::async_trait;
use domain::voice::{TelephonySession, TerminationReason};
use service::{AdmittedSipPlan, AdmittedVoicePlan, CredentialSet};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use voice_runtime::VoiceSessionControl;

use crate::backend::{LaunchError, LaunchedSession, SessionLauncher};

/// Establishes SIP calls that terminate at the edge.
pub struct SipLauncher {
    /// Credentials the trunk may require. Empty for a peer that does not authenticate.
    credentials: CredentialSet,
}

impl SipLauncher {
    #[must_use]
    pub fn new(credentials: CredentialSet) -> Self {
        Self { credentials }
    }
}

#[async_trait]
impl SessionLauncher for SipLauncher {
    /// Ready without contacting anything.
    ///
    /// There is no application endpoint to probe and no signing key to load — the two things the
    /// composed launcher checks here. A SIP peer's reachability is not a readiness fact: it is
    /// discovered by dialling, and a trunk that is down must not stop the daemon starting.
    async fn ready(&self) -> Result<(), LaunchError> {
        Ok(())
    }

    /// Refused: this launcher has no application channel to carry a call onward to.
    ///
    /// Named rather than quietly establishing a raw call instead. A caller that admitted an
    /// application route expects the call to reach it, and one that connects and goes nowhere
    /// would look like success.
    async fn launch(&self, _admitted: AdmittedVoicePlan) -> Result<LaunchedSession, LaunchError> {
        Err(LaunchError::new("application_channel_unsupported"))
    }

    async fn launch_sip(&self, admitted: AdmittedSipPlan) -> Result<LaunchedSession, LaunchError> {
        let control = VoiceSessionControl::new();
        let (completion_sender, completion) = watch::channel(None);
        let cancelled = CancellationToken::new();
        let session: Arc<dyn TelephonySession> =
            driver_sip::establish_outbound(&admitted, &self.credentials, cancelled)
                .await
                .map_err(|_| LaunchError::new("establishment_failed"))?;

        let descriptor = session.descriptor();
        let receipt = protocol::sip::SipDialEstablished {
            call: descriptor.call.as_str().to_owned(),
            session: descriptor.session.as_str().to_owned(),
            // No application channel exists, and the receipt says so rather than naming one.
            channel: None,
            state: protocol::sip::SipDialState::Established,
        };

        // The call outlives this call, so its terminal fact is published by a task that waits for
        // the driver's own answer rather than inferring one from media or signal EOF.
        let supervised = Arc::clone(&session);
        tokio::spawn(async move {
            let reason = supervised
                .wait_terminated()
                .await
                .unwrap_or(TerminationReason::TransportLost);
            completion_sender.send_replace(Some(crate::runtime::termination(reason)));
        });

        Ok(LaunchedSession {
            receipt,
            control,
            completion,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn readiness_contacts_nothing() {
        // A trunk that is down must not stop the daemon starting: reachability is discovered by
        // dialling, not by a startup probe. The composed launcher probes a credential source and an
        // application endpoint, and this launcher has neither.
        SipLauncher::new(CredentialSet::default())
            .ready()
            .await
            .expect("raw SIP readiness depends on nothing");
    }

    #[test]
    fn the_receipt_claims_no_application_channel() {
        // The protocol change this arm exists to justify: a raw call's receipt has no channel, and
        // says so, rather than naming one that does not exist.
        let receipt = protocol::sip::SipDialEstablished {
            call: "call-1".to_owned(),
            session: "session-1".to_owned(),
            channel: None,
            state: protocol::sip::SipDialState::Established,
        };
        let rendered = serde_json::to_value(&receipt).expect("a receipt renders");
        assert!(
            rendered.get("channel").is_none(),
            "an absent channel is omitted from the wire, not sent as null"
        );
    }
}
