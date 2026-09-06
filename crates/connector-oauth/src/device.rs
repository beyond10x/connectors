//! Pure RFC 8628 response validation and polling; callers own HTTP and supply every instant.

use crate::OauthError;
use zeroize::{Zeroize as _, Zeroizing};

/// Decoded device response. The device code remains private to the acquisition owner.
///
/// ```compile_fail
/// fn must_be_debug<T: std::fmt::Debug>() {}
/// must_be_debug::<connector_oauth::device::DeviceResponse>();
/// ```
pub struct DeviceResponse {
    pub device_code: Zeroizing<String>,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: Option<u64>,
}

impl Drop for DeviceResponse {
    fn drop(&mut self) {
        self.user_code.zeroize();
        self.verification_uri.zeroize();
        if let Some(uri) = &mut self.verification_uri_complete {
            uri.zeroize();
        }
    }
}

/// Bounded human instructions; deliberately no device code or formatting implementation.
pub struct DeviceInstructions<'a> {
    pub user_code: &'a str,
    pub verification_uri: &'a str,
    pub verification_uri_complete: Option<&'a str>,
}

/// A validated private device authorization.
///
/// ```compile_fail
/// fn must_be_debug<T: std::fmt::Debug>() {}
/// must_be_debug::<connector_oauth::device::DeviceAuthorization>();
/// ```
pub struct DeviceAuthorization {
    response: DeviceResponse,
    deadline_unix_ms: u64,
    first_poll_unix_ms: u64,
    interval_ms: u64,
}

impl DeviceAuthorization {
    #[must_use]
    pub fn instructions(&self) -> DeviceInstructions<'_> {
        DeviceInstructions {
            user_code: &self.response.user_code,
            verification_uri: &self.response.verification_uri,
            verification_uri_complete: self.response.verification_uri_complete.as_deref(),
        }
    }
    #[must_use]
    pub fn deadline_unix_ms(&self) -> u64 {
        self.deadline_unix_ms
    }
}

/// Validate decoded, size-bounded fields against an admitted HTTPS origin and caller time.
/// No field is inferred from another; an omitted complete URI stays absent.
pub fn validate_device(
    response: DeviceResponse,
    admitted_origin: &str,
    now_unix_ms: u64,
    session_deadline_unix_ms: u64,
) -> Result<DeviceAuthorization, OauthError> {
    let refused = OauthError::DeviceResponse;
    let origin = url::Url::parse(admitted_origin).map_err(|_| refused)?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
        || origin.path() != "/"
        || response.device_code.is_empty()
        || response.device_code.len() > 4_096
        || response
            .device_code
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
        || response.user_code.is_empty()
        || response.user_code.len() > 256
        || response.user_code.chars().any(char::is_control)
        || response.expires_in == 0
        || session_deadline_unix_ms <= now_unix_ms
    {
        return Err(refused);
    }
    for value in std::iter::once(response.verification_uri.as_str())
        .chain(response.verification_uri_complete.as_deref())
    {
        if value.is_empty()
            || value.len() > 4_096
            || value.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(refused);
        }
        let uri = url::Url::parse(value).map_err(|_| refused)?;
        if uri.scheme() != "https"
            || uri.origin() != origin.origin()
            || !uri.username().is_empty()
            || uri.password().is_some()
            || uri.fragment().is_some()
        {
            return Err(refused);
        }
    }
    let expiry = response
        .expires_in
        .checked_mul(1_000)
        .and_then(|lifetime| now_unix_ms.checked_add(lifetime))
        .ok_or(refused)?;
    let interval_ms = response
        .interval
        .unwrap_or(5)
        .checked_mul(1_000)
        .filter(|interval| *interval > 0)
        .ok_or(refused)?;
    let deadline_unix_ms = expiry.min(session_deadline_unix_ms);
    let first_poll_unix_ms = now_unix_ms
        .saturating_add(interval_ms)
        .min(deadline_unix_ms);
    Ok(DeviceAuthorization {
        response,
        deadline_unix_ms,
        first_poll_unix_ms,
        interval_ms,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollStatus {
    WaitUntil(u64),
    Ready,
    InFlight,
    Authorized,
    Denied,
    Expired,
    Cancelled,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollOutcome {
    Pending,
    SlowDown,
    TransportTimeout,
    Authorized,
    AccessDenied,
    ExpiredToken,
    Refused,
}

/// One authorization's bounded polling state. Terminal transitions retire the private code.
///
/// ```compile_fail
/// fn must_be_debug<T: std::fmt::Debug>() {}
/// must_be_debug::<connector_oauth::device::DevicePoll>();
/// ```
pub struct DevicePoll {
    authorization: Option<DeviceAuthorization>,
    deadline_unix_ms: u64,
    next_poll_unix_ms: u64,
    interval_ms: u64,
    in_flight: bool,
    last_observed_unix_ms: u64,
    terminal: Option<PollStatus>,
}
impl DevicePoll {
    #[must_use]
    pub fn new(authorization: DeviceAuthorization) -> Self {
        Self {
            deadline_unix_ms: authorization.deadline_unix_ms,
            next_poll_unix_ms: authorization.first_poll_unix_ms,
            interval_ms: authorization.interval_ms,
            authorization: Some(authorization),
            in_flight: false,
            last_observed_unix_ms: 0,
            terminal: None,
        }
    }
    pub fn status(&mut self, now_unix_ms: u64) -> PollStatus {
        if let Some(status) = self.terminal {
            return status;
        }
        self.last_observed_unix_ms = self.last_observed_unix_ms.max(now_unix_ms);
        let now_unix_ms = self.last_observed_unix_ms;
        if now_unix_ms >= self.deadline_unix_ms {
            return self.terminate(PollStatus::Expired);
        }
        if self.in_flight {
            PollStatus::InFlight
        } else if now_unix_ms < self.next_poll_unix_ms {
            PollStatus::WaitUntil(self.next_poll_unix_ms)
        } else {
            PollStatus::Ready
        }
    }
    /// Claim one polling attempt; the borrowed code cannot outlive this state owner.
    pub fn begin_poll(&mut self, now_unix_ms: u64) -> Result<&str, PollStatus> {
        let status = self.status(now_unix_ms);
        if status != PollStatus::Ready {
            return Err(status);
        }
        self.in_flight = true;
        Ok(self
            .authorization
            .as_ref()
            .expect("live poll owns authorization")
            .response
            .device_code
            .as_str())
    }
    /// Record one result. Late or unsolicited successes cannot revive terminal state.
    pub fn finish_poll(&mut self, outcome: PollOutcome, now_unix_ms: u64) -> PollStatus {
        let status = self.status(now_unix_ms);
        if self.terminal.is_some() {
            return status;
        }
        if !self.in_flight {
            return self.terminate(PollStatus::Refused);
        }
        self.in_flight = false;
        match outcome {
            PollOutcome::Authorized => return self.terminate(PollStatus::Authorized),
            PollOutcome::AccessDenied => return self.terminate(PollStatus::Denied),
            PollOutcome::ExpiredToken => return self.terminate(PollStatus::Expired),
            PollOutcome::Refused => return self.terminate(PollStatus::Refused),
            PollOutcome::Pending => {}
            PollOutcome::SlowDown => self.interval_ms = self.interval_ms.saturating_add(5_000),
            PollOutcome::TransportTimeout => self.interval_ms = self.interval_ms.saturating_mul(2),
        }
        self.next_poll_unix_ms = self
            .last_observed_unix_ms
            .saturating_add(self.interval_ms)
            .min(self.deadline_unix_ms);
        self.status(now_unix_ms)
    }
    pub fn cancel(&mut self) {
        if self.terminal.is_none() {
            self.terminate(PollStatus::Cancelled);
        }
    }
    fn terminate(&mut self, status: PollStatus) -> PollStatus {
        self.authorization = None;
        self.in_flight = false;
        self.terminal = Some(status);
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response() -> DeviceResponse {
        DeviceResponse {
            device_code: Zeroizing::new("private-device".into()),
            user_code: "ABCD-EFGH".into(),
            verification_uri: "https://gitlab.example/device".into(),
            verification_uri_complete: None,
            expires_in: 300,
            interval: None,
        }
    }
    fn authorization() -> DeviceAuthorization {
        validate_device(response(), "https://gitlab.example", 1_000, 301_000)
            .expect("valid device response")
    }

    #[test]
    fn device_instructions_do_not_invent_a_complete_uri_and_deadline_is_minimum() {
        let authorization =
            validate_device(response(), "https://gitlab.example", 1_000, 31_000).unwrap();
        assert_eq!(authorization.deadline_unix_ms(), 31_000);
        assert_eq!(authorization.instructions().verification_uri_complete, None);
        assert_eq!(authorization.instructions().user_code, "ABCD-EFGH");
        let mut response = response();
        response.verification_uri_complete =
            Some("https://gitlab.example/device?user_code=ABCD-EFGH".into());
        assert!(
            validate_device(response, "https://gitlab.example", 1_000, 601_000)
                .unwrap()
                .instructions()
                .verification_uri_complete
                .is_some()
        );
    }

    #[test]
    fn device_response_refuses_the_entire_origin_bounds_and_expiry_class() {
        type Change = fn(&mut DeviceResponse);
        let cases: &[Change] = &[
            |r| r.device_code.clear(),
            |r| *r.device_code = "a".repeat(4_097),
            |r| r.user_code.clear(),
            |r| r.user_code = "a".repeat(257),
            |r| r.user_code = "ABCD\nEFGH".into(),
            |r| r.verification_uri = "http://gitlab.example/device".into(),
            |r| r.verification_uri = "https://other.example/device".into(),
            |r| r.verification_uri = "https://user@gitlab.example/device".into(),
            |r| r.verification_uri = "https://gitlab.example/device#fragment".into(),
            |r| r.verification_uri = format!("https://gitlab.example/{}", "a".repeat(4_096)),
            |r| r.verification_uri_complete = Some("https://other.example/device".into()),
            |r| r.verification_uri_complete = Some(String::new()),
            |r| r.expires_in = 0,
            |r| r.expires_in = u64::MAX,
            |r| r.interval = Some(0),
            |r| r.interval = Some(u64::MAX),
        ];
        for change in cases {
            let mut r = response();
            change(&mut r);
            assert!(validate_device(r, "https://gitlab.example", 1_000, 31_000).is_err());
        }
        assert!(validate_device(response(), "https://gitlab.example", 1_000, 1_000).is_err());
        assert!(
            validate_device(response(), "https://gitlab.example", u64::MAX - 1, u64::MAX).is_err()
        );
    }

    #[test]
    fn device_poll_obeys_default_interval_pending_slow_down_and_timeout_backoff() {
        let mut poll = DevicePoll::new(authorization());
        assert_eq!(poll.status(1_000), PollStatus::WaitUntil(6_000));
        assert_eq!(
            poll.begin_poll(5_999).err(),
            Some(PollStatus::WaitUntil(6_000))
        );
        assert_eq!(poll.begin_poll(6_000).unwrap(), "private-device");
        assert_eq!(poll.begin_poll(6_000).err(), Some(PollStatus::InFlight));
        assert_eq!(
            poll.finish_poll(PollOutcome::Pending, 6_100),
            PollStatus::WaitUntil(11_100)
        );
        poll.begin_poll(11_100).unwrap();
        assert_eq!(
            poll.finish_poll(PollOutcome::SlowDown, 11_200),
            PollStatus::WaitUntil(21_200)
        );
        poll.begin_poll(21_200).unwrap();
        assert_eq!(
            poll.finish_poll(PollOutcome::Pending, 21_300),
            PollStatus::WaitUntil(31_300)
        );
        poll.begin_poll(31_300).unwrap();
        assert_eq!(
            poll.finish_poll(PollOutcome::TransportTimeout, 31_400),
            PollStatus::WaitUntil(51_400)
        );
    }

    #[test]
    fn caller_clock_rollback_never_shortens_the_polling_interval() {
        let mut poll = DevicePoll::new(authorization());
        poll.begin_poll(6_000).unwrap();
        assert_eq!(
            poll.finish_poll(PollOutcome::Pending, 1_000),
            PollStatus::WaitUntil(11_000)
        );
        assert_eq!(
            poll.begin_poll(10_999).err(),
            Some(PollStatus::WaitUntil(11_000))
        );
    }

    #[test]
    fn device_poll_terminal_outcomes_never_authorize_again_or_retain_a_code() {
        for (outcome, expected) in [
            (PollOutcome::Authorized, PollStatus::Authorized),
            (PollOutcome::AccessDenied, PollStatus::Denied),
            (PollOutcome::ExpiredToken, PollStatus::Expired),
            (PollOutcome::Refused, PollStatus::Refused),
        ] {
            let mut poll = DevicePoll::new(authorization());
            poll.begin_poll(6_000).unwrap();
            assert_eq!(poll.finish_poll(outcome, 6_001), expected);
            assert_eq!(poll.begin_poll(6_002).err(), Some(expected));
            assert_eq!(poll.finish_poll(PollOutcome::Pending, 6_003), expected);
        }
    }

    #[test]
    fn device_poll_expiry_cancel_and_unsolicited_response_are_terminal() {
        let mut poll = DevicePoll::new(authorization());
        assert_eq!(poll.status(301_000), PollStatus::Expired);
        assert_eq!(poll.begin_poll(301_000).err(), Some(PollStatus::Expired));
        let mut poll = DevicePoll::new(authorization());
        poll.begin_poll(6_000).unwrap();
        poll.cancel();
        assert_eq!(
            poll.finish_poll(PollOutcome::Authorized, 6_001),
            PollStatus::Cancelled
        );
        let mut poll = DevicePoll::new(authorization());
        assert_eq!(
            poll.finish_poll(PollOutcome::Authorized, 1_001),
            PollStatus::Refused
        );
        let mut r = response();
        r.interval = Some(7);
        let mut poll =
            DevicePoll::new(validate_device(r, "https://gitlab.example", 1_000, 7_000).unwrap());
        assert_eq!(poll.status(1_000), PollStatus::WaitUntil(7_000));
        assert_eq!(poll.status(7_000), PollStatus::Expired);
    }
}
