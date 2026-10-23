use objc2::{MainThreadMarker, rc::Retained};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSError, NSOperatingSystemVersion, NSProcessInfo,
};
use objc2_service_management::SMAppService;

use super::{
    LaunchReason, LoginItemError, LoginItemStatus, PROPERTY_DATA, classify_launch,
    registration_status,
};

/// Owns only the current app's service; its registration belongs to macOS.
pub(super) struct MainApp {
    service: Retained<SMAppService>,
    _main_thread: MainThreadMarker,
}

impl MainApp {
    pub(super) fn new() -> Result<Self, LoginItemError> {
        let main_thread = MainThreadMarker::new().ok_or(LoginItemError::NotMainThread)?;
        let required = NSOperatingSystemVersion {
            majorVersion: 13,
            minorVersion: 0,
            patchVersion: 0,
        };
        if !NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(required) {
            return Err(LoginItemError::UnsupportedMacOsVersion);
        }
        // SAFETY: SMAppService and mainAppService are available on macOS 13+,
        // checked above. The returned retained object identifies this process's
        // own main app, needs no arbitrary bundle identifier, and stays on the
        // main thread with this handle. No registration occurs here.
        let service = unsafe { SMAppService::mainAppService() };
        Ok(Self {
            service,
            _main_thread: main_thread,
        })
    }

    pub(super) fn status(&self) -> LoginItemStatus {
        // SAFETY: new checked OS availability, and this retained main-app
        // service cannot leave the main thread. Querying status mutates no
        // registration. Unknown future values remain explicit.
        registration_status(unsafe { self.service.status() }.0)
    }

    pub(super) fn register(&self) -> Result<LoginItemStatus, LoginItemError> {
        // SAFETY: this is the retained, available main-app service created by
        // new, used on the main thread. NSError ownership is handled by objc2.
        unsafe { self.service.registerAndReturnError() }.map_err(system_error)?;
        Ok(self.status())
    }

    pub(super) fn unregister(&self) -> Result<LoginItemStatus, LoginItemError> {
        // SAFETY: same available main-app service and thread as register.
        // Unregistering the main app does not terminate its current process.
        unsafe { self.service.unregisterAndReturnError() }.map_err(system_error)?;
        Ok(self.status())
    }

    pub(super) fn open_settings(&self) {
        // SAFETY: new established macOS 13+ and main-thread access. This API
        // takes no pointer or callback and only requests the Login Items pane.
        unsafe { SMAppService::openSystemSettingsLoginItems() };
    }
}

fn system_error(error: Retained<NSError>) -> LoginItemError {
    LoginItemError::System {
        domain: error.domain().to_string(),
        code: error.code(),
        description: error.localizedDescription().to_string(),
    }
}

pub(super) fn current_launch_reason() -> Result<LaunchReason, LoginItemError> {
    let _main_thread = MainThreadMarker::new().ok_or(LoginItemError::NotMainThread)?;
    let event = NSAppleEventManager::sharedAppleEventManager().currentAppleEvent();
    Ok(reason_for_event(event.as_deref()))
}

fn reason_for_event(event: Option<&NSAppleEventDescriptor>) -> LaunchReason {
    classify_launch(event.map(|event| {
        let marker = event
            .paramDescriptorForKeyword(PROPERTY_DATA)
            .map(|value| value.enumCodeValue());
        (event.eventClass(), event.eventID(), marker)
    }))
}

#[cfg(test)]
mod tests {
    use objc2_foundation::NSAppleEventDescriptor;

    use super::reason_for_event;
    use crate::login_item::{
        CORE_EVENT_CLASS, LAUNCHED_AS_LOGIN_ITEM, LAUNCHED_AS_SERVICE_ITEM, LaunchReason,
        OPEN_APPLICATION, PROPERTY_DATA,
    };

    #[test]
    fn synthetic_apple_events_use_the_property_data_parameter() {
        // Constructs descriptors only: no event is sent, handler installed,
        // service queried or registered, or settings window opened.
        let event = NSAppleEventDescriptor::appleEventWithEventClass_eventID_targetDescriptor_returnID_transactionID(
            CORE_EVENT_CLASS,
            OPEN_APPLICATION,
            None,
            -1,
            0,
        );
        assert_eq!(reason_for_event(Some(&event)), LaunchReason::Other);
        for (marker, expected) in [
            (LAUNCHED_AS_LOGIN_ITEM, LaunchReason::LoginItem),
            (LAUNCHED_AS_SERVICE_ITEM, LaunchReason::ServiceItem),
            (0, LaunchReason::Other),
        ] {
            let value = NSAppleEventDescriptor::descriptorWithEnumCode(marker);
            event.setParamDescriptor_forKeyword(&value, PROPERTY_DATA);
            assert_eq!(reason_for_event(Some(&event)), expected);
        }
        assert_eq!(reason_for_event(None), LaunchReason::Unknown);
    }
}
