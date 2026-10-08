//! Resolve optional graphics APIs without startup imports.
use std::{
    ffi::{CStr, c_void},
    sync::OnceLock,
};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::{DirectComposition::*, Dxgi::IDXGIDevice},
        System::{LibraryLoader::*, SystemInformation::GetSystemDirectoryW},
    },
    core::*,
};

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

pub(crate) fn system_proc(
    library: &str,
    symbol: &CStr,
) -> Option<unsafe extern "system" fn() -> isize> {
    let mut path = [0u16; 32768];
    let len = unsafe { GetSystemDirectoryW(Some(&mut path)) } as usize;
    if len == 0 || len >= path.len() {
        return None;
    }
    let mut path = path[..len].to_vec();
    path.push(b'\\' as u16);
    path.extend(library.encode_utf16());
    path.push(0);
    // A successful module reference is kept alive for cached function pointers.
    let module = unsafe { LoadLibraryW(PCWSTR(path.as_ptr())) }.ok()?;
    let function = unsafe { GetProcAddress(module, PCSTR(symbol.as_ptr().cast())) };
    if function.is_none() {
        let _ = unsafe { windows::Win32::Foundation::FreeLibrary(module) };
    }
    function
}

pub(crate) fn composition_device(device: &IDXGIDevice) -> Result<IDCompositionDevice> {
    type Function =
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT;
    static FUNCTION: OnceLock<Option<Function>> = OnceLock::new();
    let function = FUNCTION
        .get_or_init(|| {
            system_proc("dcomp.dll", c"DCompositionCreateDevice")
                .map(|p| unsafe { std::mem::transmute(p) })
        })
        .ok_or_else(|| Error::from_hresult(E_NOINTERFACE))?;
    let mut result = std::ptr::null_mut();
    unsafe {
        function(device.as_raw(), &IDCompositionDevice::IID, &mut result).ok()?;
        IDCompositionDevice::from_abi(result)
    }
}
