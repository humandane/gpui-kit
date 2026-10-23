//! Native registration of the main application as a login item.
//!
//! On macOS 13 and later, [`LoginItem`] queries and changes the current
//! application's `SMAppService.mainAppService`. It never registers a helper,
//! another application, or an arbitrary bundle identifier. Use a signed app
//! bundle; Service Management reports signing and registration errors itself.
//! Creating or dropping the handle does not change registration.
//!
//! Query [`LoginItem::status`] when showing settings and after the application
//! becomes active again: the owner can change approval in System Settings.
//! Registration success does not imply approval. The returned status is the
//! platform's current answer, not a preference cached by Kit.
//!
//! [`current_launch_reason`] inspects the Apple event being handled. Capture
//! it during the application's launch callback and retain the value. Once the
//! launch event is gone, an absent event means [`LaunchReason::Unknown`], not
//! an explicit launch. This function does not install an Apple-event handler
//! or replace GPUI's event routing.
//!
//! Native calls are restricted to the main thread. Other platforms and older
//! macOS versions return an explicit unsupported error. Tests in this module
//! use synthetic event values and never change login registration or open
//! System Settings.

use std::{error::Error, fmt, marker::PhantomData, rc::Rc};

#[cfg(target_os = "macos")]
mod macos;

/// The current platform status of the main application's login registration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoginItemStatus {
    /// The application has not been registered, or was unregistered.
    NotRegistered,
    /// Registration is approved and the application is eligible to launch.
    Enabled,
    /// Registration exists, but the owner must approve it in System Settings.
    RequiresApproval,
    /// Service Management could not find the application service.
    NotFound,
    /// The operating system returned a status this version does not recognize.
    Unknown(isize),
}

/// What the currently handled launch Apple event establishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LaunchReason {
    /// The open-application event explicitly identifies a login-item launch.
    LoginItem,
    /// The open-application event explicitly identifies a service-item launch.
    ServiceItem,
    /// An open-application event was present without either known marker.
    ///
    /// This does not establish that a person explicitly launched the app.
    Other,
    /// No current open-application event establishes the launch reason.
    Unknown,
}

/// A failure to access or change the application's login registration.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoginItemError {
    /// This platform does not provide this integration.
    UnsupportedPlatform,
    /// Main-application registration requires macOS 13 or later.
    UnsupportedMacOsVersion,
    /// Native login-item operations must run on the main thread.
    NotMainThread,
    /// Service Management rejected the operation.
    #[non_exhaustive]
    System {
        /// The native error domain.
        domain: String,
        /// The native error code within that domain.
        code: isize,
        /// The native localized explanation.
        description: String,
    },
}

impl fmt::Display for LoginItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                f.write_str("login-item integration is unavailable on this platform")
            }
            Self::UnsupportedMacOsVersion => {
                f.write_str("login-item registration requires macOS 13 or later")
            }
            Self::NotMainThread => f.write_str("login-item access requires the main thread"),
            Self::System {
                domain,
                code,
                description,
            } => write!(f, "{description} ({domain}, {code})"),
        }
    }
}

impl Error for LoginItemError {}

/// A foreground-thread handle to the running application's login item.
///
/// The platform owns registration. Dropping this handle does not unregister
/// the application. The handle is neither `Send` nor `Sync`.
pub struct LoginItem {
    #[cfg(target_os = "macos")]
    service: macos::MainApp,
    _foreground: PhantomData<Rc<()>>,
}

impl LoginItem {
    /// Opens the main application's service without changing registration.
    pub fn main_app() -> Result<Self, LoginItemError> {
        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                service: macos::MainApp::new()?,
                _foreground: PhantomData,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(LoginItemError::UnsupportedPlatform)
        }
    }

    /// Queries the platform's current registration and approval state.
    pub fn status(&self) -> Result<LoginItemStatus, LoginItemError> {
        #[cfg(target_os = "macos")]
        {
            Ok(self.service.status())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(LoginItemError::UnsupportedPlatform)
        }
    }

    /// Registers this application for future logins and returns its status.
    ///
    /// Call only after the owner chooses to enable login launch. Registration
    /// may require approval; it does not start the main application again.
    /// Errors do not imply that platform state is unchanged: query [`Self::status`]
    /// again before presenting the result.
    pub fn register(&self) -> Result<LoginItemStatus, LoginItemError> {
        #[cfg(target_os = "macos")]
        {
            self.service.register()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(LoginItemError::UnsupportedPlatform)
        }
    }

    /// Unregisters this application for future logins and returns its status.
    ///
    /// The currently running main application continues running. Call only
    /// after the owner chooses to disable login launch.
    pub fn unregister(&self) -> Result<LoginItemStatus, LoginItemError> {
        #[cfg(target_os = "macos")]
        {
            self.service.unregister()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(LoginItemError::UnsupportedPlatform)
        }
    }

    /// Asks macOS to open System Settings at Login Items.
    ///
    /// Call in response to the owner's request. Success means the request
    /// was issued, not that registration was approved or settings were shown.
    pub fn open_settings(&self) -> Result<(), LoginItemError> {
        #[cfg(target_os = "macos")]
        {
            self.service.open_settings();
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(LoginItemError::UnsupportedPlatform)
        }
    }
}

/// Inspects the current launch event on the main thread without changing it.
///
/// Capture this value during the launch callback, before handing startup work
/// to an asynchronous task. No current open-application event yields
/// [`LaunchReason::Unknown`]; it must not be treated as proof of explicit launch.
/// This inspection does not require a registered login item or macOS 13.
pub fn current_launch_reason() -> Result<LaunchReason, LoginItemError> {
    #[cfg(target_os = "macos")]
    {
        macos::current_launch_reason()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(LoginItemError::UnsupportedPlatform)
    }
}

// AppleEvents.h and AERegistry.h define these four-character codes. Keep the
// conversion here so the same classification can be tested on every platform.
#[cfg(any(target_os = "macos", test))]
const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
#[cfg(any(target_os = "macos", test))]
const OPEN_APPLICATION: u32 = u32::from_be_bytes(*b"oapp");
#[cfg(target_os = "macos")]
const PROPERTY_DATA: u32 = u32::from_be_bytes(*b"prdt");
#[cfg(any(target_os = "macos", test))]
const LAUNCHED_AS_LOGIN_ITEM: u32 = u32::from_be_bytes(*b"lgit");
#[cfg(any(target_os = "macos", test))]
const LAUNCHED_AS_SERVICE_ITEM: u32 = u32::from_be_bytes(*b"svit");

#[cfg(any(target_os = "macos", test))]
fn classify_launch(event: Option<(u32, u32, Option<u32>)>) -> LaunchReason {
    match event {
        Some((CORE_EVENT_CLASS, OPEN_APPLICATION, Some(LAUNCHED_AS_LOGIN_ITEM))) => {
            LaunchReason::LoginItem
        }
        Some((CORE_EVENT_CLASS, OPEN_APPLICATION, Some(LAUNCHED_AS_SERVICE_ITEM))) => {
            LaunchReason::ServiceItem
        }
        Some((CORE_EVENT_CLASS, OPEN_APPLICATION, _)) => LaunchReason::Other,
        _ => LaunchReason::Unknown,
    }
}

#[cfg(any(target_os = "macos", test))]
fn registration_status(value: isize) -> LoginItemStatus {
    match value {
        0 => LoginItemStatus::NotRegistered,
        1 => LoginItemStatus::Enabled,
        2 => LoginItemStatus::RequiresApproval,
        3 => LoginItemStatus::NotFound,
        other => LoginItemStatus::Unknown(other),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CORE_EVENT_CLASS, LAUNCHED_AS_LOGIN_ITEM, LAUNCHED_AS_SERVICE_ITEM, LaunchReason,
        LoginItemStatus, OPEN_APPLICATION, classify_launch, registration_status,
    };

    #[test]
    fn login_requires_the_launch_event_and_its_login_marker() {
        let event = |marker| Some((CORE_EVENT_CLASS, OPEN_APPLICATION, marker));
        assert_eq!(
            classify_launch(event(Some(LAUNCHED_AS_LOGIN_ITEM))),
            LaunchReason::LoginItem
        );
        assert_eq!(
            classify_launch(event(Some(LAUNCHED_AS_SERVICE_ITEM))),
            LaunchReason::ServiceItem
        );
        assert_eq!(classify_launch(event(None)), LaunchReason::Other);
        assert_eq!(classify_launch(event(Some(0))), LaunchReason::Other);
        assert_eq!(classify_launch(None), LaunchReason::Unknown);
        assert_eq!(
            classify_launch(Some((0, OPEN_APPLICATION, Some(LAUNCHED_AS_LOGIN_ITEM)))),
            LaunchReason::Unknown
        );
        assert_eq!(
            classify_launch(Some((
                CORE_EVENT_CLASS,
                u32::from_be_bytes(*b"rapp"),
                Some(LAUNCHED_AS_LOGIN_ITEM),
            ))),
            LaunchReason::Unknown
        );
    }

    #[test]
    fn approval_and_unknown_statuses_are_never_reported_as_enabled() {
        assert_eq!(registration_status(0), LoginItemStatus::NotRegistered);
        assert_eq!(registration_status(1), LoginItemStatus::Enabled);
        assert_eq!(registration_status(2), LoginItemStatus::RequiresApproval);
        assert_eq!(registration_status(3), LoginItemStatus::NotFound);
        assert_eq!(registration_status(4), LoginItemStatus::Unknown(4));
        assert_eq!(registration_status(-1), LoginItemStatus::Unknown(-1));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn unavailable_platform_does_not_guess_registration_or_launch_reason() {
        use super::{LoginItem, LoginItemError, current_launch_reason};

        assert!(matches!(
            LoginItem::main_app(),
            Err(LoginItemError::UnsupportedPlatform)
        ));
        assert_eq!(
            current_launch_reason(),
            Err(LoginItemError::UnsupportedPlatform)
        );
    }
}
