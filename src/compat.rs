// Windows 7 selection shared by the compatibility paths.
use std::sync::OnceLock;

pub(crate) fn legacy_mode() -> bool {
    static LEGACY: OnceLock<bool> = OnceLock::new();
    *LEGACY.get_or_init(|| {
        if std::env::var("GPUI_WIN7_COMPAT").is_ok_and(|v| v == "1") {
            return true;
        }
        let mut version = windows::Win32::System::SystemInformation::OSVERSIONINFOW::default();
        version.dwOSVersionInfoSize = std::mem::size_of_val(&version) as u32;
        let status = unsafe { windows::Wdk::System::SystemServices::RtlGetVersion(&mut version) };
        status.is_ok()
            && (version.dwMajorVersion < 6
                || (version.dwMajorVersion == 6 && version.dwMinorVersion < 2))
    })
}
