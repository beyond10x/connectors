use super::*;
use connectors_sdk::Secret;
use std::{
    fs::File,
    marker::PhantomData,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

/// Owned process, never reconstructed from a PID or readiness file. It is not
/// Send: its spawning thread must remain alive for the Linux parent-death lease.
pub struct Child {
    process: std::process::Child,
    pidfd: File,
    channel: UnixStream,
    bootstrap: Bootstrap,
    incarnation: String,
    live: bool,
    _parent_thread: PhantomData<Rc<()>>,
}
impl Child {
    pub fn spawn(config: &super::super::config::Adapter) -> Result<Self> {
        Self::spawn_until(config, Instant::now() + Duration::from_secs(10))
    }
    pub(crate) fn spawn_until(
        config: &super::super::config::Adapter,
        deadline: Instant,
    ) -> Result<Self> {
        let until = deadline.min(Instant::now() + Duration::from_secs(10));
        let executable = config
            .executable
            .open()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let executable = super::artifact::capture(executable, &config.executable.sha256, until)?;
        let (channel, child_channel) = UnixStream::pair().map_err(|_| Failure::Unavailable)?;
        channel::peer(&channel)?;
        let child_fd = child_channel.as_raw_fd();
        // SAFETY: getpid has no preconditions.
        let parent = unsafe { libc::getpid() };
        let mut command = Command::new(format!("/proc/self/fd/{}", executable.as_raw_fd()));
        command
            .args(&config.executable.args)
            .args(["--connectors-private-fd", "3"])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // SAFETY: the hook uses only async-signal-safe syscalls, captured scalar
        // values and fixed errors. Both source descriptors remain held by spawn.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0
                    || libc::getppid() != parent
                {
                    return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
                }
                if libc::dup2(child_fd, 3) < 0 || libc::fcntl(3, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut process = command.spawn().map_err(|_| Failure::Unavailable)?;
        drop(child_channel);
        drop(executable);
        // SAFETY: pidfd_open takes the just-spawned child's PID and zero flags.
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, process.id(), 0) } as i32;
        if raw < 0 {
            let _ = process.kill();
            let _ = process.wait();
            return Err(Failure::Unavailable);
        }
        // SAFETY: the successful syscall returned a new owned descriptor.
        let pidfd = unsafe { File::from_raw_fd(raw) };
        let incarnation = uuid::Uuid::new_v4().to_string();
        let nonce = uuid::Uuid::new_v4().to_string();
        // Build an owned guard before any fallible channel work.
        let mut child = Self {
            process,
            pidfd,
            channel,
            bootstrap: Bootstrap {
                instance: String::new(),
                adapter: String::new(),
                protocol: String::new(),
                configuration_revision: String::new(),
                provider_authority: String::new(),
                descriptor: String::new(),
                profiles: Vec::new(),
                requirements: Vec::new(),
            },
            incarnation: incarnation.clone(),
            live: true,
            _parent_thread: PhantomData,
        };
        channel::write(
            &mut child.channel,
            &Request::Hello {
                version: VERSION.into(),
                nonce: nonce.clone(),
                child_incarnation: incarnation.clone(),
            },
            None,
            &[],
            until,
        )?;
        let response = channel::read::<Reply>(&mut child.channel, until, false, 0)?;
        let Reply::Ready {
            version,
            nonce: returned,
            child_incarnation,
            bootstrap,
        } = response.control
        else {
            return Err(Failure::ReadinessMismatch);
        };
        bootstrap
            .validate()
            .map_err(|_| Failure::ReadinessMismatch)?;
        if version != VERSION
            || returned != nonce
            || child_incarnation != incarnation
            || bootstrap.instance != config.instance_id
            || bootstrap.adapter != config.adapter_id
            || bootstrap.protocol != config.protocol
            || bootstrap.configuration_revision != config.configuration_revision
        {
            return Err(Failure::ReadinessMismatch);
        }
        if child
            .process
            .try_wait()
            .map_err(|_| Failure::Unavailable)?
            .is_some()
        {
            return Err(Failure::Unavailable);
        }
        child.bootstrap = bootstrap;
        Ok(child)
    }
    pub fn bootstrap(&self) -> &Bootstrap {
        &self.bootstrap
    }
    pub fn incarnation(&self) -> &str {
        &self.incarnation
    }
    /// Duplicate this owned kernel handle for the owner's independent stop path.
    pub(crate) fn termination_handle(&self) -> Result<File> {
        self.pidfd.try_clone().map_err(|_| Failure::Unavailable)
    }
    pub fn running(&mut self) -> Result<bool> {
        if self.live
            && self
                .process
                .try_wait()
                .map_err(|_| Failure::Unavailable)?
                .is_some()
        {
            self.live = false;
        }
        Ok(self.live)
    }
    pub fn validate(
        &mut self,
        profile: &str,
        secret: &Secret,
        deadline_ms: u64,
    ) -> Result<Baseline> {
        self.bootstrap.profile(profile)?;
        let id = uuid::Uuid::new_v4().to_string();
        let frame = self.exchange(
            Request::Validate {
                request_id: id.clone(),
                profile: profile.to_owned(),
                deadline_ms,
            },
            Some(secret),
            &[],
            deadline_ms,
        )?;
        match frame.control {
            Reply::Validated {
                request_id,
                baseline,
            } if request_id == id && frame.document.is_empty() => Ok(baseline),
            Reply::Failed { request_id, code } if request_id == id && frame.document.is_empty() => {
                Err(code)
            }
            _ => {
                self.terminate();
                Err(Failure::Protocol)
            }
        }
    }
    /// A transport operation only. The caller must already have admitted current
    /// policy, scopes, exact material and final dispatch through the registry.
    pub fn invoke(
        &mut self,
        operation: &str,
        revision: &str,
        partition: &str,
        secret: &Secret,
        input: &[u8],
        deadline_ms: u64,
    ) -> Result<Vec<u8>> {
        if input.len() > INPUT_LIMIT || !connectors_core::valid_id(partition) {
            return Err(Failure::InvalidInput);
        }
        let requirement = self
            .bootstrap
            .requirements
            .iter()
            .find(|r| r.operation == operation)
            .ok_or(Failure::NotFound)?;
        if requirement.effect != Effect::Read {
            return Err(Failure::Unsupported);
        }
        if self.bootstrap.descriptor()?.revision != revision {
            return Err(Failure::StaleDescription);
        }
        channel::depth(input)?;
        let id = uuid::Uuid::new_v4().to_string();
        let frame = self.exchange(
            Request::Invoke {
                request_id: id.clone(),
                operation: operation.into(),
                revision: revision.into(),
                partition: partition.into(),
                deadline_ms,
            },
            Some(secret),
            input,
            deadline_ms,
        )?;
        match frame.control {
            Reply::Success { request_id } if request_id == id => {
                let checked = (|| {
                    channel::depth(&frame.document)?;
                    let value = connectors_core::read_json(&frame.document)
                        .map_err(|_| Failure::Protocol)?;
                    let descriptor = self.bootstrap.descriptor()?;
                    let operation = descriptor
                        .operation(operation)
                        .map_err(Failure::from_service)?;
                    connectors_sdk::validate(&operation.output_schema, &value)
                        .map_err(|_| Failure::Protocol)
                })();
                if let Err(error) = checked {
                    self.terminate();
                    return Err(error);
                }
                Ok(frame.document)
            }
            Reply::Failed { request_id, code } if request_id == id && frame.document.is_empty() => {
                Err(code)
            }
            _ => {
                self.terminate();
                Err(Failure::Protocol)
            }
        }
    }
    fn exchange(
        &mut self,
        request: Request,
        secret: Option<&Secret>,
        document: &[u8],
        deadline_ms: u64,
    ) -> Result<channel::Frame<Reply>> {
        if !self.live {
            return Err(Failure::Unavailable);
        }
        let duration = deadline_ms
            .checked_sub(connectors_sdk::now_ms())
            .filter(|ms| (1..=120_000).contains(ms))
            .ok_or(Failure::Timeout)?;
        let until = Instant::now() + Duration::from_millis(duration);
        let result = channel::write(&mut self.channel, &request, secret, document, until)
            .and_then(|_| channel::read(&mut self.channel, until, false, RESULT_LIMIT));
        if result.is_err()
            || matches!(
                &result,
                Ok(channel::Frame {
                    control: Reply::Failed {
                        code: Failure::Timeout | Failure::Interrupted,
                        ..
                    },
                    ..
                })
            )
        {
            self.terminate();
        }
        result
    }
    pub fn stop(&mut self, expected: &str) -> Result<()> {
        if expected != self.incarnation {
            return Err(Failure::IncarnationMismatch);
        }
        if !self.live {
            return Ok(());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let until = Instant::now() + Duration::from_secs(5);
        let response = channel::write(
            &mut self.channel,
            &Request::Stop {
                request_id: id.clone(),
            },
            None,
            &[],
            until,
        )
        .and_then(|_| channel::read::<Reply>(&mut self.channel, until, false, 0));
        let acknowledged = matches!(response,Ok(channel::Frame {control:Reply::Stopped {request_id},..}) if request_id==id);
        self.terminate();
        if acknowledged {
            Ok(())
        } else {
            Err(Failure::Unavailable)
        }
    }
    fn terminate(&mut self) {
        if self.live {
            // SAFETY: pidfd is an owned handle to this child, never a numeric PID
            // restored from metadata. No siginfo is supplied.
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pidfd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                );
            }
            let _ = self.process.wait();
            self.live = false;
        }
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::config::{Adapter, Executable, Restart, Startup};
    use sha2::{Digest, Sha256};

    fn bootstrap() -> Bootstrap {
        Bootstrap {
            instance: "fixture".into(), adapter: "fixture".into(), protocol: "v1alpha1".into(),
            configuration_revision: "fixture-config".into(), provider_authority: "fixture-authority".into(),
            descriptor: serde_json::json!({
                "version":"v1alpha1", "instance":"fixture", "adapter":"fixture", "revision":"fixture-descriptor",
                "configuration_schema":{"type":"object"},
                "operations":[{"id":"read", "description":"Fictional read", "contract":"fixture/1", "profile":"fixture",
                    "input_schema":{"type":"object"}, "output_schema":{"type":"object","required":["value"],"properties":{"value":{"type":"boolean"}},"additionalProperties":false}}]
            }).to_string(),
            profiles: vec![Profile {
                id:"fixture".into(),revision:"fixture-profile".into(),purpose:registry::Purpose::DelegatedUser,
                subject:registry::Subject::User,scheme:"http_bearer".into(),capability:"http-bearer".into(),
                minimum_scopes:BTreeSet::new(),evidence_lifetime_ms:60_000,
                fields:vec![EntryField{name:"token".into(),label:"Fictional credential".into(),max_bytes:100}],
            }],
            requirements:vec![Requirement{operation:"read".into(),profile:"fixture".into(),scopes:BTreeSet::new(),effect:Effect::Read}],
        }
    }

    // The ordinary test invocation does nothing. Child::spawn re-enters just
    // this test through libtest's filter syntax, with the real private fd and
    // process-ownership path. No shell, helper executable or provider is needed.
    #[test]
    fn protocol_fixture() {
        let args: Vec<_> = std::env::args().collect();
        if !args.iter().any(|a| a == "--connectors-private-fd") {
            return;
        }
        let mode = args
            .iter()
            .find(|a| a.starts_with("fixture-mode-"))
            .unwrap();
        // SAFETY: only the spawned fixture path owns inherited descriptor three.
        let mut stream = unsafe { UnixStream::from_raw_fd(3) };
        channel::peer(&stream).unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        let hello = channel::read::<Request>(&mut stream, until, false, 0).unwrap();
        let Request::Hello {
            version,
            nonce,
            child_incarnation,
        } = hello.control
        else {
            panic!("expected hello")
        };
        channel::write(
            &mut stream,
            &Reply::Ready {
                version,
                nonce,
                child_incarnation,
                bootstrap: bootstrap(),
            },
            None,
            &[],
            until,
        )
        .unwrap();
        let request = channel::read::<Request>(&mut stream, until, true, INPUT_LIMIT).unwrap();
        let Request::Invoke { request_id, .. } = request.control else {
            panic!("expected invocation")
        };
        let (id, body) = match mode.as_str() {
            "fixture-mode-json" => (request_id, b"{\"value\":}".as_slice()),
            "fixture-mode-schema" => (request_id, br#"{"value":"wrong-type"}"#.as_slice()),
            "fixture-mode-identity" => ("another-request".into(), br#"{"value":true}"#.as_slice()),
            _ => panic!("unknown fixture mode"),
        };
        channel::write(
            &mut stream,
            &Reply::Success { request_id: id },
            None,
            body,
            until,
        )
        .unwrap();
        // The parent must kill and reap this exact child after the bad reply.
        std::thread::sleep(Duration::from_secs(30));
    }

    #[test]
    fn malformed_replies_terminate_and_reap_only_the_owned_child() {
        let path = std::env::current_exe().unwrap();
        let hash = hex::encode(Sha256::digest(std::fs::read(&path).unwrap()));
        for mode in ["json", "schema", "identity"] {
            let config = Adapter {
                permissions: Default::default(),
                instance_id: "fixture".into(),
                adapter_id: "fixture".into(),
                configuration_revision: "fixture-config".into(),
                protocol: "v1alpha1".into(),
                startup: Startup::OnDemand,
                restart: Restart::Never,
                executable: Executable {
                    path: path.clone(),
                    sha256: hash.clone(),
                    args: vec![
                        "--exact".into(),
                        "local::runtime::process::tests::protocol_fixture".into(),
                        "--".into(),
                        format!("fixture-mode-{mode}"),
                    ],
                },
            };
            let mut child = Child::spawn(&config).unwrap();
            assert!(child.process.try_wait().unwrap().is_none());
            assert_eq!(
                child.invoke(
                    "read",
                    "fixture-descriptor",
                    "partition",
                    &Secret(b"fictional".to_vec()),
                    b"{}",
                    connectors_sdk::now_ms() + 2000
                ),
                Err(Failure::Protocol)
            );
            assert!(!child.live);
            assert!(child.process.try_wait().unwrap().is_some());
        }
    }
}
