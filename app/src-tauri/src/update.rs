//! Whether a newer Kuchu is out. Installed from the Microsoft Store, Windows can say, and can install it: Windows does the asking, and
//! Kuchu connects to nothing. Any other copy could only find out by going online, which Kuchu never does, so its button opens the
//! website instead, in the browser.

use std::time::Duration;

use serde::Serialize;
use tokio::sync::oneshot;

use crate::install;

/// How long the Store has to answer: past this it counts as not answering, and the button opens its page.
const STORE_WAIT: Duration = Duration::from_secs(20);

/// What is known about a newer Kuchu.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Update {
    /// Not installed from the Store: the website says what the latest is.
    Site,
    /// The Store has nothing newer.
    Current,
    /// The Store has a newer version, and which, if it says.
    Ready { version: Option<String> },
    /// The Store did not answer: its own page for Kuchu can say.
    Unknown,
}

/// A package's version as people read it: the Store's fourth number is always 0.
fn version_name(major: u16, minor: u16, build: u16) -> String {
    format!("{major}.{minor}.{build}")
}

#[cfg(windows)]
mod store {
    use windows::Services::Store::{StoreContext, StorePackageUpdateState};
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    use super::{version_name, Update};

    /// The Store's view of this app, on a thread of its own.
    fn context() -> windows::core::Result<StoreContext> {
        // Safety: called once on a thread made for this, which ends with it.
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        StoreContext::GetDefault()
    }

    pub fn check() -> windows::core::Result<Update> {
        let updates = context()?.GetAppAndOptionalStorePackageUpdatesAsync()?.join()?;
        if updates.Size()? == 0 {
            return Ok(Update::Current);
        }
        let version = updates.GetAt(0).and_then(|u| u.Package()).and_then(|p| p.Id()).and_then(|id| id.Version()).ok();
        Ok(Update::Ready { version: version.map(|v| version_name(v.Major, v.Minor, v.Build)) })
    }

    /// Downloads and installs what the Store has, the click on "Update now" being the person's say. Windows closes Kuchu to finish, and
    /// starts it again (`install::restart_after_update`). False if Windows would not do it unattended.
    pub fn install() -> windows::core::Result<bool> {
        let store = context()?;
        let updates = store.GetAppAndOptionalStorePackageUpdatesAsync()?.join()?;
        if updates.Size()? == 0 || !store.CanSilentlyDownloadStorePackageUpdates()? {
            return Ok(false);
        }
        let done = store.TrySilentDownloadAndInstallStorePackageUpdatesAsync(&updates)?.join()?;
        Ok(done.OverallState()? == StorePackageUpdateState::Completed)
    }
}

#[cfg(not(windows))]
mod store {
    use super::Update;

    pub fn check() -> Result<Update, ()> {
        Ok(Update::Site)
    }

    pub fn install() -> Result<bool, ()> {
        Ok(false)
    }
}

/// Runs `ask` on a thread of its own, within `STORE_WAIT`: a Store that hangs costs an answer, not Kuchu.
async fn on_its_own_thread<T: Send + 'static>(ask: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(ask());
    });
    tokio::time::timeout(STORE_WAIT, rx).await.ok()?.ok()
}

/// What is known about a newer Kuchu right now.
pub async fn status() -> Update {
    if !install::packaged() {
        return Update::Site;
    }
    on_its_own_thread(|| store::check().unwrap_or(Update::Unknown)).await.unwrap_or(Update::Unknown)
}

/// Installs what the Store has. False if it did not, and the Store's own page has to take over. Most of the time this does not return:
/// Windows closes Kuchu to finish.
pub async fn install() -> bool {
    install::packaged() && on_its_own_thread(|| store::install().unwrap_or(false)).await.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_store_version_reads_as_people_write_it() {
        assert_eq!(version_name(0, 1, 8), "0.1.8");
    }

    #[test]
    fn the_page_is_told_what_is_known_in_a_shape_it_can_read() {
        assert_eq!(serde_json::to_value(Update::Site).unwrap(), serde_json::json!({ "state": "site" }));
        assert_eq!(serde_json::to_value(Update::Ready { version: Some("0.1.8".into()) }).unwrap(), serde_json::json!({ "state": "ready", "version": "0.1.8" }));
    }

    /// The call that decides whether the button can fall back: outside a package the Store has no app to speak of, as it has none when
    /// something goes wrong for a person. It has to answer, whatever it says, and not hang.
    #[cfg(windows)]
    #[tokio::test]
    async fn asking_the_store_from_outside_a_package_answers_and_does_not_hang() {
        let answer = on_its_own_thread(|| format!("{:?}", store::check())).await;
        println!("the Store says: {answer:?}");
        assert!(answer.is_some());
    }

    #[tokio::test]
    async fn a_copy_not_from_the_store_goes_to_the_website_and_has_nothing_to_install() {
        assert_eq!(status().await, Update::Site);
        assert!(!install().await);
    }
}
