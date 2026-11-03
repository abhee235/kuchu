//! What the relay and Kuchu share: where they meet, how long each step may take, and how the pipe is opened.
//!
//! The relay sends one line of JSON, `{"v":1,"event":…,"payload":…}`. For a permission request Kuchu answers with
//! one line: the decision object Claude Code expects (`{"behavior":"allow"}`, `{"behavior":"deny","message":…}`, with
//! `updatedInput` or `updatedPermissions` when they apply), or closes the connection to leave the question to the
//! terminal.
//!
//! The pipe is opened only through `open`, which speaks to nothing but a pipe this account made: a pipe's name can be
//! taken by any program on the machine, and whatever answers under Kuchu's name would otherwise see every event and
//! answer every request.

use std::time::{Duration, Instant};

/// The protocol's version, in every message.
pub const VERSION: u32 = 1;

/// How long the relay tries to reach Kuchu; after that it gives up without a word.
pub const CONNECT: Duration = Duration::from_millis(300);
/// The whole life of the relay for an ordinary event.
pub const EVENT: Duration = Duration::from_secs(2);
/// How long the relay waits for a person's answer to a permission request. The hook's own timeout in settings.json
/// is longer (`HOOK_TIMEOUT_PERMISSION`), and Kuchu's is shorter (`DECIDE`), so each side gives up before the one it
/// waits on.
pub const PERMISSION: Duration = Duration::from_secs(110);
/// How long Kuchu waits for a click.
pub const DECIDE: Duration = Duration::from_secs(108);
/// How long Kuchu has to put a permission card on the screen; otherwise the request goes back to the terminal.
pub const SHOWN: Duration = Duration::from_millis(800);
/// Hook timeouts written into settings.json (seconds).
pub const HOOK_TIMEOUT: u32 = 10;
pub const HOOK_TIMEOUT_PERMISSION: u32 = 120;

/// The longest line either side reads.
pub const MAX_LINE: usize = 1 << 20;

/// Where the two meet. On Windows a named pipe per user; elsewhere a socket in a folder only the user can open.
#[cfg(windows)]
pub fn address() -> String {
    format!(r"\\.\pipe\kuchu-{}", user_tag())
}

#[cfg(not(windows))]
pub fn address() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    format!("{home}/.kuchu/kuchu.sock")
}

/// The user's name as it can appear in a pipe's name: letters and digits kept, anything else as its code point.
#[cfg(windows)]
fn user_tag() -> String {
    let name = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_string() } else { format!("_{:x}", c as u32) })
        .collect()
}

/// The open connection to Kuchu.
#[cfg(windows)]
pub type Stream = std::fs::File;
#[cfg(unix)]
pub type Stream = std::os::unix::net::UnixStream;

/// What is found at Kuchu's address.
pub enum Pipe {
    /// Kuchu's own pipe, made by this account, open.
    Kuchu(Stream),
    /// Every instance is taken for the moment: Kuchu is there, with its hands full.
    Busy,
    /// Nothing listens.
    Nobody,
    /// Something under Kuchu's name that this account did not make, or that will not open: never spoken to.
    Someone,
}

/// Opens Kuchu's pipe, trying again while it is busy within `budget`. Only a pipe this account made is returned open.
pub fn open(budget: Duration) -> Pipe {
    open_at(&address(), budget)
}

#[cfg(windows)]
pub fn open_at(name: &str, budget: Duration) -> Pipe {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION;

    const ERROR_FILE_NOT_FOUND: i32 = 2;
    const ERROR_PIPE_BUSY: i32 = 231;
    let deadline = Instant::now() + budget;
    loop {
        // With SECURITY_IDENTIFICATION the pipe's server learns who the client is and nothing more. Without it, the
        // server may act as the client, so a program holding the name could do what the person can.
        let opened = std::fs::OpenOptions::new().read(true).write(true).security_qos_flags(SECURITY_IDENTIFICATION).open(name);
        match opened {
            Ok(file) => return if owner::is_me(file.as_raw_handle()) { Pipe::Kuchu(file) } else { Pipe::Someone },
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                if Instant::now() >= deadline {
                    return Pipe::Busy;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => return Pipe::Nobody,
            Err(_) => return Pipe::Someone,
        }
    }
}

#[cfg(unix)]
pub fn open_at(path: &str, budget: Duration) -> Pipe {
    use std::io::ErrorKind;
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::UnixStream;

    let deadline = Instant::now() + budget;
    loop {
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_socket() => {}
            Ok(_) => return Pipe::Someone,
            Err(e) if e.kind() == ErrorKind::NotFound => return Pipe::Nobody,
            Err(_) => return Pipe::Someone,
        }
        match UnixStream::connect(path) {
            Ok(stream) => return if owner::is_me(&stream) { Pipe::Kuchu(stream) } else { Pipe::Someone },
            // The listener's queue is full for the moment.
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Pipe::Busy;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            // A socket file left behind, with nobody listening.
            Err(e) if matches!(e.kind(), ErrorKind::ConnectionRefused | ErrorKind::NotFound) => return Pipe::Nobody,
            Err(_) => return Pipe::Someone,
        }
    }
}

/// Who made the pipe: the owner of the pipe object, against the account this process runs as.
#[cfg(windows)]
mod owner {
    use std::os::windows::io::RawHandle;
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, ERROR_SUCCESS, HANDLE};
    use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT};
    use windows_sys::Win32::Security::{EqualSid, GetTokenInformation, TokenUser, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, TOKEN_QUERY, TOKEN_USER};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    /// Whether the pipe behind `handle` was made by this account.
    pub fn is_me(handle: RawHandle) -> bool {
        me().is_some_and(|me| is(handle, me.sid()))
    }

    /// Whether the pipe behind `handle` is owned by the account `sid`.
    pub fn is(handle: RawHandle, sid: PSID) -> bool {
        // Safety: a valid handle, out-pointers for what is asked, and the descriptor freed once read.
        unsafe {
            let (mut owner, mut descriptor): (PSID, PSECURITY_DESCRIPTOR) = (null_mut(), null_mut());
            let err = GetSecurityInfo(handle as HANDLE, SE_KERNEL_OBJECT, OWNER_SECURITY_INFORMATION, &mut owner, null_mut(), null_mut(), null_mut(), &mut descriptor);
            let same = err == ERROR_SUCCESS && !owner.is_null() && EqualSid(owner, sid) != 0;
            if !descriptor.is_null() {
                LocalFree(descriptor);
            }
            same
        }
    }

    /// The account this process runs as: the user of its token, kept in the buffer the system fills.
    pub struct Me(Vec<u8>);

    impl Me {
        pub fn sid(&self) -> PSID {
            // Safety: the buffer holds a TOKEN_USER whose SID points inside the same buffer.
            unsafe { (*(self.0.as_ptr() as *const TOKEN_USER)).User.Sid }
        }
    }

    pub fn me() -> Option<Me> {
        // Safety: the token is asked for its size, then read into a buffer of that size, then closed.
        unsafe {
            let mut token: HANDLE = null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return None;
            }
            let mut len = 0u32;
            GetTokenInformation(token, TokenUser, null_mut(), 0, &mut len);
            let mut buf = vec![0u8; len as usize];
            let read = len > 0 && GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), len, &mut len) != 0;
            CloseHandle(token);
            read.then_some(Me(buf))
        }
    }
}

/// Who is at the other end of the socket, against the account this process runs as.
#[cfg(unix)]
mod owner {
    use std::os::unix::io::AsRawFd;
    use std::os::unix::net::UnixStream;

    pub fn is_me(stream: &UnixStream) -> bool {
        // Safety: a connected socket's descriptor; getuid cannot fail.
        peer_uid(stream.as_raw_fd()).is_some_and(|uid| uid == unsafe { libc::getuid() })
    }

    #[cfg(target_os = "linux")]
    fn peer_uid(fd: i32) -> Option<libc::uid_t> {
        // Safety: an out-struct of the size the call expects.
        unsafe {
            let mut cred: libc::ucred = std::mem::zeroed();
            let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
            (libc::getsockopt(fd, libc::SOL_SOCKET, libc::SO_PEERCRED, (&mut cred as *mut libc::ucred).cast(), &mut len) == 0).then_some(cred.uid)
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn peer_uid(fd: i32) -> Option<libc::uid_t> {
        // Safety: out-values for the call.
        unsafe {
            let (mut uid, mut gid): (libc::uid_t, libc::gid_t) = (0, 0);
            (libc::getpeereid(fd, &mut uid, &mut gid) == 0).then_some(uid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_side_gives_up_before_the_one_it_waits_on() {
        assert!(DECIDE < PERMISSION);
        assert!(PERMISSION < Duration::from_secs(HOOK_TIMEOUT_PERMISSION as u64));
        assert!(EVENT < Duration::from_secs(HOOK_TIMEOUT as u64));
    }

    #[cfg(windows)]
    #[test]
    fn a_pipe_per_user_whatever_the_name() {
        std::env::set_var("USERNAME", "Ana María");
        assert_eq!(address(), r"\\.\pipe\kuchu-Ana_20Mar_eda");
    }

    #[cfg(windows)]
    #[test]
    fn only_a_pipe_this_account_made_is_kuchus() {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::Security::{CreateWellKnownSid, WinWorldSid, SECURITY_MAX_SID_SIZE};
        use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
        use windows_sys::Win32::System::Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE};

        // A pipe of this test's own, made as Kuchu makes its own: by this account.
        let name = format!(r"\\.\pipe\kuchu-test-{}", std::process::id());
        let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
        let server = unsafe { CreateNamedPipeW(wide.as_ptr(), PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_TYPE_BYTE, 1, 4096, 4096, 0, std::ptr::null()) };
        assert_ne!(server, INVALID_HANDLE_VALUE);
        let Pipe::Kuchu(client) = open_at(&name, Duration::ZERO) else { panic!("this account's pipe should open as Kuchu's") };

        // The same pipe is not Everyone's.
        let mut everyone = vec![0u8; SECURITY_MAX_SID_SIZE as usize];
        let mut len = SECURITY_MAX_SID_SIZE;
        assert_ne!(unsafe { CreateWellKnownSid(WinWorldSid, std::ptr::null_mut(), everyone.as_mut_ptr().cast(), &mut len) }, 0);
        assert!(owner::is(client.as_raw_handle(), owner::me().unwrap().sid()));
        assert!(!owner::is(client.as_raw_handle(), everyone.as_mut_ptr().cast()));

        drop(client);
        unsafe { CloseHandle(server) };
        assert!(matches!(open_at(&name, Duration::ZERO), Pipe::Nobody));
    }

    #[cfg(unix)]
    #[test]
    fn only_a_socket_this_account_listens_on_is_kuchus() {
        let dir = std::env::temp_dir().join(format!("kuchu-wire-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("kuchu.sock");
        let at = path.display().to_string();
        assert!(matches!(open_at(&at, Duration::ZERO), Pipe::Nobody));
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        assert!(matches!(open_at(&at, Duration::ZERO), Pipe::Kuchu(_)));
        drop(listener);
        // The file stays, nobody listens.
        assert!(matches!(open_at(&at, Duration::ZERO), Pipe::Nobody));
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, "").unwrap();
        assert!(matches!(open_at(&at, Duration::ZERO), Pipe::Someone));
        let _ = std::fs::remove_dir_all(dir);
    }
}
