//! "Open at Login", through the system's login-item service (macOS 13+).

use layoutsmart_core::ports::LoginItem;
use objc2_service_management::SMAppService;

pub struct LoginService;

impl LoginItem for LoginService {
    fn register(&self) -> bool {
        // SAFETY: plain calls on the main app's service object.
        unsafe { SMAppService::mainAppService().registerAndReturnError() }.is_ok()
    }
}
