//! Where a session runs, so a card can go to it. Claude Code does not say, and nothing in a message is trusted to: the system names the
//! process at the other end of the pipe, and its parents lead to the app the session runs in (a relay's parents are the hook's shell,
//! then Claude Code, then whatever started that). Only apps on a short list count. All that is ever done with one is to bring its own
//! window forward: nothing is started, and nothing a session sent takes any part in it but the project's name, to tell a window by its title.

/// A running process, as the system lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Proc {
    pub pid: u32,
    /// Its program's file name, in lower case.
    pub exe: String,
}

/// The app a session runs in.
#[derive(Clone, Debug, PartialEq)]
pub struct Host {
    /// What the card calls it.
    pub name: String,
    pid: u32,
    exe: String,
}

/// What windows a process has on the screen.
#[derive(Clone, Copy, Default)]
pub struct Windows {
    pub any: bool,
    pub console: bool,
}

/// The apps a session can run in, by program. Windows Terminal hosts its shells, so it is their parent; an editor's chat panel and its
/// built-in terminal both run under it.
const APPS: &[(&str, &str)] = &[
    ("code.exe", "VS Code"),
    ("code - insiders.exe", "VS Code Insiders"),
    ("cursor.exe", "Cursor"),
    ("windsurf.exe", "Windsurf"),
    ("windowsterminal.exe", "Windows Terminal"),
    ("mintty.exe", "Git Bash"),
];
/// Claude Code's own program, and the desktop app's, which has the same name: only the app has a window.
const CLAUDE: &str = "claude.exe";
const SHELLS: &[&str] = &["powershell.exe", "pwsh.exe", "cmd.exe", "bash.exe", "nu.exe"];

/// The app in a chain of processes, nearest first. `windows` says what a process has on the screen; `terminal` is Windows Terminal's
/// process if one runs: a shell started outside it can be handed over to it, and then has neither it for a parent nor a window of its own.
pub fn choose(chain: &[Proc], windows: &dyn Fn(u32) -> Windows, terminal: Option<u32>) -> Option<Host> {
    for (at, p) in chain.iter().enumerate() {
        if let Some((_, name)) = APPS.iter().find(|(exe, _)| *exe == p.exe) {
            // An editor is many processes of one program, and only the outermost has the windows.
            let top = chain[at..].iter().take_while(|o| o.exe == p.exe).last().unwrap_or(p);
            return windows(top.pid).any.then(|| Host { name: (*name).into(), pid: top.pid, exe: top.exe.clone() });
        }
        if p.exe == CLAUDE && windows(p.pid).any {
            return Some(Host { name: "Claude".into(), pid: p.pid, exe: p.exe.clone() });
        }
        if windows(p.pid).console {
            return Some(Host { name: "Terminal".into(), pid: p.pid, exe: p.exe.clone() });
        }
    }
    let claude = chain.iter().position(|p| p.exe == CLAUDE)?;
    let in_a_shell = chain[claude..].iter().any(|p| SHELLS.contains(&p.exe.as_str()));
    terminal.filter(|_| in_a_shell).map(|pid| Host { name: "Windows Terminal".into(), pid, exe: "windowsterminal.exe".into() })
}

#[cfg(windows)]
mod system {
    use windows_sys::core::BOOL;
    use windows_sys::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HWND, INVALID_HANDLE_VALUE, LPARAM};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_BASIC_INFORMATION, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_MOUSE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, GW_OWNER,
        SW_RESTORE,
    };

    use super::{choose, Host, Proc, Windows};

    /// How far up a chain goes: a relay is a few parents from any app, and a chain must end.
    const MOST: usize = 16;

    /// What a process's own handle says of it, which holds while anything keeps it open, even once it has ended: its program's file name
    /// in lower case, its parent, and when it started.
    fn inspect(pid: u32) -> Option<(String, u32, u64)> {
        // Safety: the handle is closed below; each out-value is the size the call is told.
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return None;
            }
            let mut basic: PROCESS_BASIC_INFORMATION = std::mem::zeroed();
            let mut path = [0u16; 512];
            let mut len = path.len() as u32;
            let mut times: [FILETIME; 4] = std::mem::zeroed();
            let [created, exited, kernel, user] = &mut times;
            let read = NtQueryInformationProcess(process, ProcessBasicInformation, (&mut basic as *mut PROCESS_BASIC_INFORMATION).cast(), std::mem::size_of::<PROCESS_BASIC_INFORMATION>() as u32, std::ptr::null_mut()) == 0
                && QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut len) != 0
                && GetProcessTimes(process, created, exited, kernel, user) != 0;
            CloseHandle(process);
            if !read {
                return None;
            }
            let path = String::from_utf16_lossy(&path[..len as usize]);
            let exe = path.rsplit('\\').next().unwrap_or(&path).to_lowercase();
            Some((exe, basic.InheritedFromUniqueProcessId as u32, (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime)))
        }
    }

    /// The process and its parents, nearest first. Taken the moment a relay connects: it writes and ends at once, and its parents with
    /// it. A parent started after its child is not its parent: that one has ended, and its number been given to something else.
    pub fn chain(pid: u32) -> Vec<Proc> {
        let mut out: Vec<Proc> = Vec::new();
        let mut at = pid;
        let mut child_started = u64::MAX;
        while out.len() < MOST {
            let Some((exe, parent, started)) = inspect(at) else { break };
            if started > child_started || out.iter().any(|p| p.pid == at) {
                break;
            }
            out.push(Proc { pid: at, exe });
            child_started = started;
            at = parent;
        }
        out
    }

    /// Windows Terminal's process, if one runs.
    fn terminal() -> Option<u32> {
        // Safety: the snapshot is closed below, and the entry is the size it says.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return None;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut found = None;
            let mut more = Process32FirstW(snapshot, &mut entry) != 0;
            while more && found.is_none() {
                let end = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
                if String::from_utf16_lossy(&entry.szExeFile[..end]).eq_ignore_ascii_case("windowsterminal.exe") {
                    found = Some(entry.th32ProcessID);
                }
                more = Process32NextW(snapshot, &mut entry) != 0;
            }
            CloseHandle(snapshot);
            found
        }
    }

    /// A window on the screen: its handle, the process that has it, its title and whether it is a console's.
    struct Seen {
        window: HWND,
        pid: u32,
        title: String,
        console: bool,
    }

    /// The windows a person can see, the frontmost first. An owned window (a dialog) goes with its owner.
    fn seen() -> Vec<Seen> {
        unsafe extern "system" fn each(window: HWND, list: LPARAM) -> BOOL {
            // Safety: `list` is the vector below, for as long as EnumWindows runs; the buffers are the size they say.
            unsafe {
                if IsWindowVisible(window) != 0 && GetWindow(window, GW_OWNER).is_null() {
                    let mut title = [0u16; 256];
                    let len = GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32);
                    if len > 0 {
                        let mut pid = 0u32;
                        GetWindowThreadProcessId(window, &mut pid);
                        let mut class = [0u16; 32];
                        let n = GetClassNameW(window, class.as_mut_ptr(), class.len() as i32);
                        let console = String::from_utf16_lossy(&class[..n.max(0) as usize]) == "ConsoleWindowClass";
                        (*(list as *mut Vec<Seen>)).push(Seen { window, pid, title: String::from_utf16_lossy(&title[..len as usize]), console });
                    }
                }
            }
            1
        }
        let mut list: Vec<Seen> = Vec::new();
        // Safety: the callback only runs during this call, while `list` is alive.
        unsafe { EnumWindows(Some(each), &mut list as *mut Vec<Seen> as LPARAM) };
        list
    }

    /// The app in a chain taken a moment ago.
    pub fn find(chain: &[Proc]) -> Option<Host> {
        let windows = seen();
        let on_screen = |pid: u32| Windows { any: windows.iter().any(|w| w.pid == pid), console: windows.iter().any(|w| w.pid == pid && w.console) };
        choose(chain, &on_screen, terminal())
    }

    /// Brings the app's window forward: the one with the project in its title if it has several. False if the app has gone, or
    /// Windows would not.
    pub fn raise(host: &Host, project: &str) -> bool {
        if inspect(host.pid).is_none_or(|(exe, _, _)| exe != host.exe) {
            return false;
        }
        // Its own process first, then any other of the same program: Windows Terminal can run its windows in several, and a shell handed
        // over to it says nothing of which. Among those, the window with the project in its title; a title says only what shows now.
        let windows = seen();
        let (own, others): (Vec<&Seen>, Vec<&Seen>) = windows.iter().filter(|w| w.pid == host.pid || inspect(w.pid).is_some_and(|(exe, _, _)| exe == host.exe)).partition(|w| w.pid == host.pid);
        let project = project.to_lowercase();
        let names = |w: &&&Seen| !project.is_empty() && w.title.to_lowercase().contains(&project);
        let Some(chosen) = own.iter().find(names).or_else(|| others.iter().find(names)).or_else(|| own.first()).or_else(|| others.first()) else { return false };
        // Safety: a window handle from just now; an empty mouse input moves nothing. Windows lets the program that took the last input
        // bring a window forward, and a click on a card goes to the page's own process, not this one: hence the empty input.
        unsafe {
            if IsIconic(chosen.window) != 0 {
                ShowWindow(chosen.window, SW_RESTORE);
            }
            let mut input: INPUT = std::mem::zeroed();
            input.r#type = INPUT_MOUSE;
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
            SetForegroundWindow(chosen.window);
            GetForegroundWindow() == chosen.window
        }
    }
}

#[cfg(not(windows))]
mod system {
    use super::{Host, Proc};

    pub fn chain(_pid: u32) -> Vec<Proc> {
        Vec::new()
    }

    pub fn find(_chain: &[Proc]) -> Option<Host> {
        None
    }

    pub fn raise(_host: &Host, _project: &str) -> bool {
        false
    }
}

pub use system::{chain, find, raise};

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(names: &[&str]) -> Vec<Proc> {
        names.iter().enumerate().map(|(i, exe)| Proc { pid: 100 + i as u32, exe: (*exe).into() }).collect()
    }

    fn found(names: &[&str], with_window: &[u32], with_console: &[u32], terminal: Option<u32>) -> Option<(String, u32)> {
        let windows = |pid: u32| Windows { any: with_window.contains(&pid) || with_console.contains(&pid), console: with_console.contains(&pid) };
        choose(&chain(names), &windows, terminal).map(|h| (h.name, h.pid))
    }

    /// As measured on Windows 11: each chain from the relay up, what started it last.
    const PANEL: &[&str] = &["kuchu-relay.exe", "bash.exe", "bash.exe", "claude.exe", "code.exe", "code.exe", "explorer.exe"];
    const TERMINAL: &[&str] = &["kuchu-relay.exe", "bash.exe", "claude.exe", "powershell.exe", "windowsterminal.exe"];
    const DESKTOP: &[&str] = &["kuchu-relay.exe", "bash.exe", "claude.exe", "claude.exe", "explorer.exe"];
    const CONSOLE: &[&str] = &["kuchu-relay.exe", "bash.exe", "claude.exe", "cmd.exe", "explorer.exe"];
    const HANDED_OVER: &[&str] = &["kuchu-relay.exe", "bash.exe", "claude.exe", "powershell.exe", "explorer.exe"];

    #[test]
    fn a_session_in_an_editor_goes_to_the_process_with_its_windows() {
        assert_eq!(found(PANEL, &[105, 106], &[], None), Some(("VS Code".into(), 105)));
    }

    #[test]
    fn a_session_in_a_terminal_goes_to_the_terminal() {
        assert_eq!(found(TERMINAL, &[104], &[], Some(104)), Some(("Windows Terminal".into(), 104)));
        assert_eq!(found(CONSOLE, &[104], &[103], None), Some(("Terminal".into(), 103)));
    }

    #[test]
    fn a_shell_handed_over_to_windows_terminal_finds_it_though_it_is_not_a_parent() {
        assert_eq!(found(HANDED_OVER, &[104], &[], Some(900)), Some(("Windows Terminal".into(), 900)));
        assert_eq!(found(HANDED_OVER, &[104], &[], None), None);
    }

    #[test]
    fn only_the_desktop_app_is_claude_with_a_window() {
        assert_eq!(found(DESKTOP, &[103, 104], &[], None), Some(("Claude".into(), 103)));
    }

    #[test]
    fn a_session_kuchu_started_or_one_from_an_app_not_on_the_list_has_nowhere_to_go() {
        assert_eq!(found(&["kuchu-relay.exe", "bash.exe", "claude.exe", "kuchu.exe", "explorer.exe"], &[103, 104], &[], Some(900)), None);
        assert_eq!(found(&["kuchu-relay.exe", "claude.exe", "someapp.exe"], &[102], &[], None), None);
    }

    #[test]
    fn an_app_with_no_window_on_the_screen_is_nowhere_to_go() {
        assert_eq!(found(PANEL, &[], &[], None), None);
    }
}
