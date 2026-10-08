//! Redirect the two legacy COM imports used by this backend to ole32.
//! Both entry points predate combase; their modern Rust bindings select combase.
//! Providing the MSVC import slots here keeps those bindings and their ABI intact.
use std::{ffi::c_void, sync::OnceLock};
use windows::{Win32::Foundation::E_NOTIMPL, core::HRESULT};

unsafe extern "system" fn free_memory(memory: *const c_void) {
    type Function = unsafe extern "system" fn(*const c_void);
    static FUNCTION: OnceLock<Function> = OnceLock::new();
    let function = FUNCTION.get_or_init(|| {
        let address = crate::compat::system_proc("ole32.dll", c"CoTaskMemFree")
            .expect("ole32.dll must provide CoTaskMemFree");
        unsafe { std::mem::transmute(address) }
    });
    unsafe { function(memory) };
}

unsafe extern "system" fn create_marshaler(
    outer: *mut c_void,
    marshaler: *mut *mut c_void,
) -> HRESULT {
    type Function = unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT;
    static FUNCTION: OnceLock<Option<Function>> = OnceLock::new();
    let function = FUNCTION.get_or_init(|| {
        crate::compat::system_proc("ole32.dll", c"CoCreateFreeThreadedMarshaler")
            .map(|address| unsafe { std::mem::transmute(address) })
    });
    match function {
        Some(function) => unsafe { function(outer, marshaler) },
        None => {
            if !marshaler.is_null() {
                unsafe { *marshaler = std::ptr::null_mut() };
            }
            E_NOTIMPL
        }
    }
}

#[used]
#[unsafe(no_mangle)]
static __imp_CoTaskMemFree: unsafe extern "system" fn(*const c_void) = free_memory;

#[used]
#[unsafe(no_mangle)]
static __imp_CoCreateFreeThreadedMarshaler: unsafe extern "system" fn(
    *mut c_void,
    *mut *mut c_void,
) -> HRESULT = create_marshaler;

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{
        Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize},
        core::{IUnknown, Interface},
    };

    #[test]
    fn legacy_com_slots_use_real_ole32_functions() {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }.unwrap();
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe { CoUninitialize() };
            }
        }
        let _apartment = Apartment;
        type Allocate = unsafe extern "system" fn(usize) -> *mut c_void;
        let allocate: Allocate = unsafe {
            std::mem::transmute(crate::compat::system_proc("ole32.dll", c"CoTaskMemAlloc").unwrap())
        };
        let memory = unsafe { allocate(128) };
        assert!(!memory.is_null());
        unsafe {
            std::ptr::write_bytes(memory, 0xa5, 128);
            __imp_CoTaskMemFree(memory)
        };
        let mut marshaler = std::ptr::null_mut();
        unsafe { __imp_CoCreateFreeThreadedMarshaler(std::ptr::null_mut(), &mut marshaler).ok() }
            .unwrap();
        assert!(!marshaler.is_null());
        drop(unsafe { IUnknown::from_raw(marshaler) });
    }
}
