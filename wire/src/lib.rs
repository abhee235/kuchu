//! What the relay and Kuchu share: where they meet, and how long each step may take.
//!
//! The relay sends one line of JSON, `{"v":1,"event":…,"payload":…}`. For a permission request Kuchu answers with
//! one line: the decision object Claude Code expects (`{"behavior":"allow"}`, `{"behavior":"deny","message":…}`, with
//! `updatedInput` or `updatedPermissions` when they apply), or closes the connection to leave the question to the
//! terminal.

use std::time::Duration;

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
}
