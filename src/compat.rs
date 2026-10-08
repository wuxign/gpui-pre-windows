//! Resolve optional Windows APIs without importing absent DLLs at startup.
use std::{
    ffi::{CStr, c_void},
    sync::OnceLock,
};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::{DirectComposition::*, Dxgi::IDXGIDevice, Gdi::*},
        System::{LibraryLoader::*, SystemInformation::GetSystemDirectoryW},
        UI::{HiDpi::*, WindowsAndMessaging::*},
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

fn system_dpi() -> u32 {
    unsafe {
        let dc = GetDC(None);
        if dc.is_invalid() {
            return 96;
        }
        let dpi = GetDeviceCaps(Some(dc), LOGPIXELSX);
        ReleaseDC(None, dc);
        if dpi > 0 { dpi as u32 } else { 96 }
    }
}

pub(crate) fn window_dpi(hwnd: HWND) -> u32 {
    type Function = unsafe extern "system" fn(HWND) -> u32;
    static FUNCTION: OnceLock<Option<Function>> = OnceLock::new();
    if !legacy_mode() {
        if let Some(function) = FUNCTION.get_or_init(|| {
            system_proc("user32.dll", c"GetDpiForWindow").map(|p| unsafe { std::mem::transmute(p) })
        }) {
            let dpi = unsafe { function(hwnd) };
            if dpi > 0 {
                return dpi;
            }
        }
    }
    system_dpi()
}

pub(crate) fn monitor_scale(monitor: HMONITOR) -> f32 {
    type Function =
        unsafe extern "system" fn(HMONITOR, MONITOR_DPI_TYPE, *mut u32, *mut u32) -> HRESULT;
    static FUNCTION: OnceLock<Option<Function>> = OnceLock::new();
    if !legacy_mode() {
        if let Some(function) = FUNCTION.get_or_init(|| {
            system_proc("shcore.dll", c"GetDpiForMonitor")
                .map(|p| unsafe { std::mem::transmute(p) })
        }) {
            let (mut x, mut y) = (0, 0);
            if unsafe { function(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) }.is_ok() && x > 0 {
                return x as f32 / 96.;
            }
        }
    }
    system_dpi() as f32 / 96.
}

pub(crate) fn metrics_for_dpi(index: SYSTEM_METRICS_INDEX, dpi: u32) -> i32 {
    type Function = unsafe extern "system" fn(SYSTEM_METRICS_INDEX, u32) -> i32;
    static FUNCTION: OnceLock<Option<Function>> = OnceLock::new();
    if !legacy_mode() {
        if let Some(function) = FUNCTION.get_or_init(|| {
            system_proc("user32.dll", c"GetSystemMetricsForDpi")
                .map(|p| unsafe { std::mem::transmute(p) })
        }) {
            return unsafe { function(index, dpi) };
        }
    }
    // GetSystemMetrics is already expressed in system-DPI pixels on Win7.
    unsafe { GetSystemMetrics(index) }
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
