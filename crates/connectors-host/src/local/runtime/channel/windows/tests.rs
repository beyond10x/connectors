//! The Windows channel on the runner alone: who the pipe admits, how its
//! peer is identified, and that every read and write keeps its deadline.
use super::super::{read, read_with_cancel, write};
use super::*;
use crate::local::runtime::Request;
use std::time::Instant;
use windows_sys::Win32::{
    Foundation::ERROR_ACCESS_DENIED,
    Security::{
        Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT},
        CreateWellKnownSid, DACL_SECURITY_INFORMATION, GetAce, ImpersonateAnonymousToken,
        OWNER_SECURITY_INFORMATION, PSID, RevertToSelf, SECURITY_MAX_SID_SIZE, WinLocalSystemSid,
    },
    System::Threading::GetCurrentThread,
};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(2)
}

/// Both ends of a fresh private pair, as pipes.
fn ends() -> (Pipe, Pipe) {
    let (server, client) = pair().unwrap();
    (server, Pipe::from_owned(client).unwrap())
}

fn frame(control: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    for size in [control.len(), 0, 0] {
        frame.extend_from_slice(&(size as u32).to_be_bytes());
    }
    frame.extend_from_slice(control);
    frame
}

const CONTROL: &[u8] =
    br#"{"kind":"validate","request_id":"test","profile":"profile","deadline_ms":1}"#;

/// The pipe's access control list names the current user alone, who also
/// owns it: no LocalSystem, Administrators or Everyone entry, as the default
/// pipe security would add.
#[test]
fn a_pipe_grants_the_current_user_alone() {
    let (server, _client) = pair().unwrap();
    let mut owner: PSID = std::ptr::null_mut();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut descriptor: *mut c_void = std::ptr::null_mut();
    // SAFETY: a live pipe handle and writable out pointers.
    let status = unsafe {
        GetSecurityInfo(
            server.raw(),
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            std::ptr::null_mut(),
            &mut dacl,
            std::ptr::null_mut(),
            &mut descriptor,
        )
    };
    assert_eq!(status, 0);
    let user = current_user().unwrap();
    // SAFETY: valid SIDs.
    assert_ne!(unsafe { EqualSid(owner, user.as_psid()) }, 0, "owner");
    assert!(!dacl.is_null());
    // SAFETY: a non-null DACL living as long as the descriptor.
    let count = unsafe { (*dacl).AceCount };
    assert_eq!(count, 1, "one entry");
    let mut ace: *mut c_void = std::ptr::null_mut();
    // SAFETY: index 0 is below the entry count.
    assert_ne!(unsafe { GetAce(dacl, 0, &mut ace) }, 0);
    let entry = ace as *const ACCESS_ALLOWED_ACE;
    // SAFETY: an allow entry lays out Header, Mask, then the SID at SidStart.
    let (kind, sid) = unsafe {
        (
            (*entry).Header.AceType,
            std::ptr::addr_of!((*entry).SidStart) as PSID,
        )
    };
    assert_eq!(kind, 0, "an allow entry");
    // SAFETY: valid SIDs.
    assert_ne!(
        unsafe { EqualSid(sid, user.as_psid()) },
        0,
        "the current user"
    );
    // SAFETY: GetSecurityInfo allocated the descriptor with LocalAlloc.
    unsafe { windows_sys::Win32::Foundation::LocalFree(descriptor) };
}

/// A connection made under another identity is refused at connect: this
/// thread impersonates the anonymous token, whose SID the pipe does not
/// grant, and the open is denied. The same open as the current user succeeds.
#[test]
fn a_pipe_admits_its_own_user_and_refuses_another_sid_at_connect() {
    let name = wide(&format!(
        r"\\.\pipe\connectors-test-{}",
        uuid::Uuid::new_v4()
    ));
    let _server = create_instance(&name, true, 4).unwrap();
    let refused = std::thread::spawn({
        let name = name.clone();
        move || {
            // SAFETY: impersonates on this thread only, reverted before return.
            assert_ne!(unsafe { ImpersonateAnonymousToken(GetCurrentThread()) }, 0);
            let result = open_client(&name).map(drop);
            // SAFETY: ends this thread's impersonation.
            assert_ne!(unsafe { RevertToSelf() }, 0);
            result
        }
    })
    .join()
    .unwrap();
    assert_eq!(
        refused.unwrap_err().raw_os_error(),
        Some(ERROR_ACCESS_DENIED as i32)
    );
    let admitted = open_client(&name).unwrap();
    assert!(!inheritable(admitted.as_raw_handle() as HANDLE).unwrap());
}

/// The peer's token must carry the expected SID: the same process is
/// refused against LocalSystem's SID, the identity check a pipe of another
/// user would fail, and admitted against the current user's.
#[test]
fn a_peer_whose_token_is_another_sid_is_refused() {
    let (server, _client) = ends();
    let peer = peer(&server).unwrap();
    // SAFETY: no preconditions.
    assert_eq!(peer.pid, unsafe { GetCurrentProcessId() });
    let mut storage = vec![0u32; (SECURITY_MAX_SID_SIZE as usize).div_ceil(4)];
    let mut size = SECURITY_MAX_SID_SIZE;
    // SAFETY: the buffer holds SECURITY_MAX_SID_SIZE bytes, passed as its size.
    assert_ne!(
        unsafe {
            CreateWellKnownSid(
                WinLocalSystemSid,
                std::ptr::null_mut(),
                storage.as_mut_ptr().cast(),
                &mut size,
            )
        },
        0
    );
    let system = storage.as_ptr() as PSID;
    assert!(matches!(
        admit(&peer.process, system),
        Err(Failure::Unavailable)
    ));
    admit(&peer.process, current_user().unwrap().as_psid()).unwrap();
}

/// A pair has one instance: no second client can connect to it, and the
/// client end this process opened is the recorded peer on both sides.
#[test]
fn a_pair_has_one_instance_and_its_peer_is_this_process() {
    let (server, client) = ends();
    // SAFETY: no preconditions.
    let own = unsafe { GetCurrentProcessId() };
    assert_eq!(peer(&server).unwrap().pid, own);
    assert_eq!(peer(&client).unwrap().pid, own);
    assert!(server.server_end().unwrap());
    assert!(!client.server_end().unwrap());
}

/// A pipe of the same name created first by anyone else is never joined.
#[test]
fn an_existing_pipe_name_is_never_joined_as_its_first_instance() {
    let name = wide(&format!(
        r"\\.\pipe\connectors-test-{}",
        uuid::Uuid::new_v4()
    ));
    let _first = create_instance(&name, true, 4).unwrap();
    assert!(create_instance(&name, true, 4).is_err());
}

/// A reply written just before its writer closed is read whole, with and
/// without cancellation, then end of file at once.
#[test]
fn a_reply_sent_just_before_the_peer_closed_is_read_whole() {
    for cancelled in [false, true] {
        let (mut reader, mut writer) = ends();
        writer
            .write_within(&frame(CONTROL), Duration::from_secs(2))
            .unwrap();
        drop(writer);
        let cancel = || Ok(());
        let read = read_with_cancel::<Request>(
            &mut reader,
            deadline(),
            false,
            0,
            cancelled.then_some(&cancel as &dyn Fn() -> Result<()>),
        );
        assert!(read.is_ok(), "cancellable: {cancelled}");
        let started = Instant::now();
        assert!(matches!(
            read_with_cancel::<Request>(&mut reader, deadline(), false, 0, None),
            Err(Failure::Unavailable)
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
    // The other direction: the server end closes after writing.
    let (mut writer, mut reader) = ends();
    writer
        .write_within(&frame(CONTROL), Duration::from_secs(2))
        .unwrap();
    drop(writer);
    assert!(read::<Request>(&mut reader, deadline(), false, 0).is_ok());
}

/// A read of a silent peer ends at its deadline; a stalled partial frame
/// keeps the original deadline; cancellation is observed while waiting.
#[test]
fn every_read_ends_by_its_deadline() {
    let (mut reader, mut writer) = ends();
    let started = Instant::now();
    assert!(matches!(
        read::<Request>(&mut reader, started + Duration::from_millis(100), false, 0),
        Err(Failure::Timeout)
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
    writer
        .write_within(&[0, 0, 0], Duration::from_secs(1))
        .unwrap();
    let until = Instant::now() + Duration::from_millis(50);
    assert!(matches!(
        read::<Request>(&mut reader, until, false, 0),
        Err(Failure::Timeout)
    ));
    writer.write_within(&[10], Duration::from_secs(1)).unwrap();
    assert!(matches!(
        read::<Request>(&mut reader, until, false, 0),
        Err(Failure::Timeout)
    ));
    let (mut reader, _writer) = ends();
    let started = Instant::now();
    let cancel = || {
        if started.elapsed() >= Duration::from_millis(100) {
            Err(Failure::Interrupted)
        } else {
            Ok(())
        }
    };
    assert!(matches!(
        read_with_cancel::<Request>(
            &mut reader,
            started + Duration::from_secs(5),
            false,
            0,
            Some(&cancel)
        ),
        Err(Failure::Interrupted)
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
}

/// A write to a peer that has closed fails at once as the channel's
/// `Unavailable`, the counterpart of `EPIPE` under `MSG_NOSIGNAL`; a write
/// that the peer never drains ends at its deadline.
#[test]
fn a_write_to_a_closed_or_stalled_peer_ends() {
    let (mut writer, reader) = ends();
    drop(reader);
    let started = Instant::now();
    assert!(matches!(
        write(
            &mut writer,
            &serde_json::json!({"kind":"stop"}),
            None,
            &[],
            deadline()
        ),
        Err(Failure::Unavailable)
    ));
    assert!(started.elapsed() < Duration::from_secs(1));
    let (mut writer, _reader) = ends();
    let document = vec![b' '; 4 * BUFFER as usize];
    let started = Instant::now();
    assert!(matches!(
        write(
            &mut writer,
            &serde_json::json!({"kind":"stop"}),
            None,
            &document,
            started + Duration::from_millis(200)
        ),
        Err(Failure::Timeout)
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
}

/// An inherited pipe handle is taken over as one that is not inheritable,
/// and the inherited value is closed; anything but a pipe refuses.
#[test]
fn an_adopted_handle_is_not_inheritable_and_the_inherited_one_is_closed() {
    use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
    let (_server, client) = pair().unwrap();
    let raw = client.as_raw_handle() as HANDLE;
    // SAFETY: a live owned handle.
    assert_ne!(
        unsafe { SetHandleInformation(raw, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) },
        0
    );
    assert!(inheritable(raw).unwrap());
    let value = raw as usize;
    std::mem::forget(client);
    let adopted = adopt(value).unwrap();
    assert!(!inheritable(adopted.as_raw_handle() as HANDLE).unwrap());
    // `DUPLICATE_CLOSE_SOURCE` within one process frees the inherited slot,
    // and the handle table may hand that same value to the duplicate. Either
    // the value names nothing, or it is the adopted handle itself, which is
    // not inheritable: the inherited handle no longer exists.
    assert!(
        inheritable(value as HANDLE).is_err() || adopted.as_raw_handle() as usize == value,
        "the inherited value is closed"
    );
    Pipe::from_owned(adopted).unwrap();
    let file = tempfile::tempfile().unwrap();
    let value = file.as_raw_handle() as usize;
    std::mem::forget(file);
    assert!(Pipe::from_owned(adopt(value).unwrap()).is_err());
    assert!(matches!(adopt(0), Err(Failure::InvalidInput)));
}

/// The recorded parent of a test process is the process that started it.
#[test]
fn this_process_has_a_parent() {
    assert!(parent_process_id().is_some_and(|pid| pid != 0));
}

/// An idle wait returns once bytes arrive, and once the peer has closed.
#[test]
fn an_idle_wait_returns_on_bytes_and_on_close() {
    let (reader, mut writer) = ends();
    let waiter = std::thread::spawn(move || {
        wait_readable(&reader).unwrap();
        reader
    });
    std::thread::sleep(Duration::from_millis(50));
    writer.write_within(&[1], Duration::from_secs(1)).unwrap();
    waiter.join().unwrap();
    let (reader, writer) = ends();
    drop(writer);
    let started = Instant::now();
    wait_readable(&reader).unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
}

/// owner.md, "Pipe peer": a CLI in an elevated session has the same user SID
/// as the owner a session that is not elevated started, so it reaches that
/// owner by the same pipe name. The mandatory integrity policy (no write-up)
/// denies the medium-integrity owner `PROCESS_DUP_HANDLE` on the elevated
/// client, and the integrity levels differ: the owner refuses that client by
/// name (`SESSION_MISMATCH`, answered as `lifecycle_conflict`), never as no
/// peer, so it can answer it instead of closing the stream without a reply.
/// The owner is played by this thread impersonating a medium-integrity copy
/// of this process's token, the elevated client by this process. Meaningful
/// only where the test process is elevated, as on the hosted runner;
/// elsewhere it returns early.
#[test]
fn a_medium_integrity_owner_refuses_an_elevated_client_of_its_user_by_name() {
    use windows_sys::Win32::Security::{
        Authorization::ConvertStringSidToSidW, DuplicateTokenEx, GetSidSubAuthority,
        GetSidSubAuthorityCount, ImpersonateLoggedOnUser, RevertToSelf, SID_AND_ATTRIBUTES,
        SecurityImpersonation, SetTokenInformation, TOKEN_ADJUST_DEFAULT, TOKEN_DUPLICATE,
        TOKEN_IMPERSONATE, TOKEN_MANDATORY_LABEL, TokenImpersonation, TokenIntegrityLevel,
    };
    /// SECURITY_MANDATORY_HIGH_RID.
    const HIGH: u32 = 0x3000;
    /// SE_GROUP_INTEGRITY.
    const INTEGRITY: u32 = 0x20;
    fn level(token: HANDLE) -> u32 {
        let mut buffer = vec![0u64; 16];
        let mut length = 0u32;
        // SAFETY: the buffer is 128 bytes, passed as its size, and aligned.
        assert_ne!(
            unsafe {
                GetTokenInformation(
                    token,
                    TokenIntegrityLevel,
                    buffer.as_mut_ptr().cast(),
                    (buffer.len() * 8) as u32,
                    &mut length,
                )
            },
            0
        );
        // SAFETY: a successful query wrote a TOKEN_MANDATORY_LABEL whose SID
        // follows it in the buffer.
        unsafe {
            let label = &*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL);
            let count = *GetSidSubAuthorityCount(label.Label.Sid);
            *GetSidSubAuthority(label.Label.Sid, u32::from(count) - 1)
        }
    }
    let mut own: HANDLE = std::ptr::null_mut();
    // SAFETY: this process's pseudo handle and a writable out handle.
    assert_ne!(
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY | TOKEN_DUPLICATE, &mut own,) },
        0
    );
    // SAFETY: OpenProcessToken returned a new owned handle.
    let own = unsafe { OwnedHandle::from_raw_handle(own as _) };
    if level(own.as_raw_handle() as HANDLE) < HIGH {
        return;
    }
    let mut medium: HANDLE = std::ptr::null_mut();
    // SAFETY: a live token opened with TOKEN_DUPLICATE and a writable out handle.
    assert_ne!(
        unsafe {
            DuplicateTokenEx(
                own.as_raw_handle() as HANDLE,
                TOKEN_QUERY | TOKEN_IMPERSONATE | TOKEN_ADJUST_DEFAULT,
                std::ptr::null(),
                SecurityImpersonation,
                TokenImpersonation,
                &mut medium,
            )
        },
        0
    );
    // SAFETY: DuplicateTokenEx returned a new owned handle.
    let medium = unsafe { OwnedHandle::from_raw_handle(medium as _) };
    let text: Vec<u16> = "S-1-16-8192".encode_utf16().chain(Some(0)).collect();
    let mut sid: PSID = std::ptr::null_mut();
    // SAFETY: a NUL-terminated SID string and a writable out SID, which this
    // test never frees.
    assert_ne!(
        unsafe { ConvertStringSidToSidW(text.as_ptr(), &mut sid) },
        0
    );
    let label = TOKEN_MANDATORY_LABEL {
        Label: SID_AND_ATTRIBUTES {
            Sid: sid,
            Attributes: INTEGRITY,
        },
    };
    // SAFETY: a live token opened with TOKEN_ADJUST_DEFAULT and a label of
    // the size passed, its SID included.
    assert_ne!(
        unsafe {
            SetTokenInformation(
                medium.as_raw_handle() as HANDLE,
                TokenIntegrityLevel,
                (&label as *const TOKEN_MANDATORY_LABEL).cast(),
                std::mem::size_of::<TOKEN_MANDATORY_LABEL>() as u32 + GetLengthSid(sid),
            )
        },
        0
    );
    assert!(level(medium.as_raw_handle() as HANDLE) < HIGH);
    let (server, _client) = ends();
    // SAFETY: a live impersonation token; reverted below on every path.
    assert_ne!(
        unsafe { ImpersonateLoggedOnUser(medium.as_raw_handle() as HANDLE) },
        0
    );
    let refused = peer(&server).err();
    // SAFETY: ends this thread's impersonation.
    unsafe { RevertToSelf() };
    assert_eq!(
        refused,
        Some(SESSION_MISMATCH),
        "a medium-integrity owner did not refuse an elevated client of its own user by name"
    );
}
