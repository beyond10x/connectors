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
//! # The call is audible on the machine that placed it
//!
//! Establishing alone left the media going nowhere: RTP flowed to a session nobody read. So the
//! launcher binds the session to this host's speaker and microphone through `voice-local-audio`,
//! which names no protocol — it carries a [`TelephonySession`], and RTVBP produces one of those
//! too.
//!
//! A host with no sound stack binds `driver_audio::NullAudioDevice` instead, which is chosen by
//! `driver_audio::local_device` rather than branched on here. The call still connects; nobody hears
//! it, and the deployment said so by not having a device.
//!
//! **The binding is not allowed to take the call down.** If the device refuses — no microphone, a
//! stack that disappeared between probe and use — the call stays up and unbound. Refusing to
//! establish a working call because the local speaker is busy would be the wrong trade for a
//! telephony operation whose contract is the call, not the audio.

use std::sync::Arc;

use async_trait::async_trait;
use domain::audio::AudioDevice;
use domain::voice::{TelephonySession, TerminationReason};
use service::{AdmittedSipPlan, AdmittedVoicePlan, CredentialSet};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use voice_runtime::VoiceSessionControl;

use crate::backend::{LaunchError, LaunchedSession, SessionLauncher};

/// Establishes SIP calls that terminate at the edge, audible on this machine.
pub struct SipLauncher {
    /// Credentials the trunk may require. Empty for a peer that does not authenticate.
    credentials: CredentialSet,
    /// The speaker and microphone a call is carried to. Real where this host has a sound stack,
    /// silent where it does not — the launcher does not know which, by construction.
    device: Arc<dyn AudioDevice>,
}

impl SipLauncher {
    /// Bind the trunk credentials and this host's own audio device.
    #[must_use]
    pub fn new(credentials: CredentialSet) -> Self {
        Self::with_device(credentials, driver_audio::local_device(None))
    }

    /// Bind a device the caller chose.
    ///
    /// The seam a headless deployment and every test use: pass
    /// `driver_audio::NullAudioDevice` and a call runs end to end with no sound stack present.
    #[must_use]
    pub fn with_device(credentials: CredentialSet, device: Arc<dyn AudioDevice>) -> Self {
        Self {
            credentials,
            device,
        }
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

        // Carry the media to this machine. A device refusal leaves the call up and unbound rather
        // than tearing down a working call over a busy speaker — the operation's contract is the
        // call, and the binding is what makes it audible, not what makes it succeed.
        let binding = voice_local_audio::bind(Arc::clone(&session), self.device.as_ref()).ok();

        // The call outlives this call, so its terminal fact is published by a task that waits for
        // the driver's own answer rather than inferring one from media or signal EOF.
        let supervised = Arc::clone(&session);
        tokio::spawn(async move {
            let reason = supervised
                .wait_terminated()
                .await
                .unwrap_or(TerminationReason::TransportLost);
            // Released on the driver's terminal fact, so the recorder never outlives the call it
            // was opened for.
            if let Some(binding) = binding {
                binding.stop();
            }
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
    async fn a_host_with_no_sound_stack_still_composes_a_launcher() {
        // The deployment seam: `local_device` answers with silence rather than failing, so a server
        // with no sound card composes the same launcher a workstation does. If this ever became a
        // refusal, every headless deployment would lose `sip.dial` entirely.
        let launcher = SipLauncher::new(CredentialSet::default());
        launcher
            .ready()
            .await
            .expect("composition never depends on a sound card");
        assert!(
            launcher.device.description().stack.is_none()
                || !launcher.device.description().path.is_empty(),
            "the bound device always describes itself"
        );
    }

    #[tokio::test]
    async fn a_chosen_device_is_the_one_bound() {
        // The seam a headless deployment and every test use, asserted rather than assumed.
        let launcher = SipLauncher::with_device(
            CredentialSet::default(),
            Arc::new(driver_audio::NullAudioDevice::new()),
        );
        assert_eq!(launcher.device.description().path, "null");
        assert_eq!(launcher.device.description().stack, None);
    }

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
