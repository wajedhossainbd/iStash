//! Open at login, through macOS's own login item list (SMAppService). iStash then shows in System Settings >
//! General > Login Items, where it can also be turned off.

use objc2_service_management::{SMAppService, SMAppServiceStatus};

fn status_word(s: SMAppServiceStatus) -> &'static str {
    match s {
        SMAppServiceStatus::Enabled => "on",
        SMAppServiceStatus::RequiresApproval => "approve",
        _ => "off",
    }
}

/// "on", "off", or "approve" when macOS is waiting for the user to allow it in System Settings.
#[tauri::command]
pub fn login_status() -> &'static str {
    // SAFETY: plain ServiceManagement calls on the app's own login item.
    status_word(unsafe { SMAppService::mainAppService().status() })
}

#[tauri::command]
pub fn set_login(on: bool) -> Result<&'static str, String> {
    // SAFETY: as above.
    let svc = unsafe { SMAppService::mainAppService() };
    let r = unsafe { if on { svc.registerAndReturnError() } else { svc.unregisterAndReturnError() } };
    let now = status_word(unsafe { svc.status() });
    // Turning off something already off reports an error; only a state that did not change is a failure.
    match r {
        Err(e) if (now == "on") != on && now != "approve" => Err(format!("macOS didn’t change the login item ({}). Nothing was changed.", e.code())),
        _ => Ok(now),
    }
}
