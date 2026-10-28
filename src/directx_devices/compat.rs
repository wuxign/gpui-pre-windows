//! DXGI initialization and software fallback for the compatibility backend.
use anyhow::{Context, Result};
use windows::Win32::{
    Foundation::HMODULE,
    Graphics::{
        Direct3D::{
            D3D_DRIVER_TYPE_UNKNOWN, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL,
            D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
        },
        Direct3D11::{
            D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_DEBUG, D3D11_SDK_VERSION,
            D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext,
        },
        Dxgi::{
            CreateDXGIFactory1, DXGI_ERROR_NOT_FOUND, IDXGIAdapter, IDXGIAdapter1, IDXGIDevice,
            IDXGIFactory1,
        },
    },
};
use windows::core::Interface;

use super::DirectXDevices;

#[inline]
pub(super) fn check_debug_layer_available() -> bool {
    #[cfg(debug_assertions)]
    {
        use windows::{
            Win32::Graphics::Dxgi::IDXGIInfoQueue,
            core::{GUID, Interface},
        };
        type Function = unsafe extern "system" fn(
            u32,
            *const GUID,
            *mut *mut std::ffi::c_void,
        ) -> windows::core::HRESULT;
        if crate::compat::legacy_mode() {
            return false;
        }
        let Some(function) = crate::compat::system_proc("dxgidebug.dll", c"DXGIGetDebugInterface1")
        else {
            return false;
        };
        let function: Function = unsafe { std::mem::transmute(function) };
        let mut result = std::ptr::null_mut();
        if unsafe { function(0, &IDXGIInfoQueue::IID, &mut result) }.is_err() {
            return false;
        }
        if !result.is_null() {
            drop(unsafe { IDXGIInfoQueue::from_raw(result) });
        }
        true
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}

#[inline]
pub(super) fn get_dxgi_factory(_: bool) -> Result<IDXGIFactory1> {
    // DXGI 1.1 is available on Windows 7. Newer operations query Factory2
    // at their call sites, rather than requiring Factory6 at startup.
    unsafe { Ok(CreateDXGIFactory1()?) }
}

#[inline]
pub(super) fn get_adapter(
    dxgi_factory: &IDXGIFactory1,
    debug_layer_available: bool,
) -> Result<(
    IDXGIAdapter1,
    ID3D11Device,
    ID3D11DeviceContext,
    D3D_FEATURE_LEVEL,
)> {
    let mut hardware_error = None;
    for adapter_index in 0.. {
        let adapter: IDXGIAdapter1 = match unsafe { dxgi_factory.EnumAdapters(adapter_index) } {
            Ok(adapter) => adapter.cast()?,
            Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(error) => {
                hardware_error = Some(anyhow::Error::from(error).context("Enumerating adapters"));
                break;
            }
        };
        if let Ok(desc) = unsafe { adapter.GetDesc1() } {
            let gpu_name = String::from_utf16_lossy(&desc.Description)
                .trim_matches(char::from(0))
                .to_string();
            log::info!("Using GPU: {}", gpu_name);
        }
        // Check to see whether the adapter supports Direct3D 11 and create
        // the device if it does.
        let mut context: Option<ID3D11DeviceContext> = None;
        let mut feature_level = D3D_FEATURE_LEVEL::default();
        match get_device(
            Some(&adapter),
            Some(&mut context),
            Some(&mut feature_level),
            debug_layer_available,
        ) {
            Ok(device) => return Ok((adapter, device, context.unwrap(), feature_level)),
            Err(error) => {
                log::warn!("Adapter {adapter_index} cannot render GPUI: {error:#}");
                hardware_error = Some(error);
            }
        }
    }

    log::warn!("No compatible hardware device; trying WARP software rendering");
    get_software_adapter(debug_layer_available).map_err(|software_error| {
        anyhow::anyhow!(
            "No compatible hardware Direct3D device ({}); WARP software fallback failed: {software_error:#}",
            hardware_error.map_or_else(|| "no adapters were enumerated".to_string(), |error| format!("{error:#}")),
        )
    })
}

fn get_software_adapter(
    debug: bool,
) -> Result<(
    IDXGIAdapter1,
    ID3D11Device,
    ID3D11DeviceContext,
    D3D_FEATURE_LEVEL,
)> {
    let mut context = None;
    let mut level = D3D_FEATURE_LEVEL::default();
    let device = get_device(None, Some(&mut context), Some(&mut level), debug)
        .context("Creating WARP device at Direct3D feature level 10.1 or higher")?;
    let dxgi_device: IDXGIDevice = device.cast()?;
    let adapter = unsafe { dxgi_device.GetAdapter() }?.cast()?;
    Ok((
        adapter,
        device,
        context.context("WARP device returned no immediate context")?,
        level,
    ))
}

#[inline]
fn get_device(
    adapter: Option<&IDXGIAdapter1>,
    context: Option<*mut Option<ID3D11DeviceContext>>,
    feature_level: Option<*mut D3D_FEATURE_LEVEL>,
    debug_layer_available: bool,
) -> Result<ID3D11Device> {
    let legacy_levels = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_1];
    let modern_levels = [
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
    ];
    get_device_with_levels(
        adapter,
        context,
        feature_level,
        debug_layer_available,
        if crate::compat::legacy_mode() {
            &legacy_levels
        } else {
            &modern_levels
        },
    )
}

fn get_device_with_levels(
    adapter: Option<&IDXGIAdapter1>,
    context: Option<*mut Option<ID3D11DeviceContext>>,
    feature_level: Option<*mut D3D_FEATURE_LEVEL>,
    debug_layer_available: bool,
    levels: &[D3D_FEATURE_LEVEL],
) -> Result<ID3D11Device> {
    let adapter = adapter
        .map(|adapter| adapter.cast::<IDXGIAdapter>())
        .transpose()?;
    let mut device: Option<ID3D11Device> = None;
    let device_flags = if debug_layer_available {
        D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_DEBUG
    } else {
        D3D11_CREATE_DEVICE_BGRA_SUPPORT
    };
    unsafe {
        D3D11CreateDevice(
            adapter.as_ref(),
            if adapter.is_some() {
                D3D_DRIVER_TYPE_UNKNOWN
            } else {
                D3D_DRIVER_TYPE_WARP
            },
            HMODULE::default(),
            device_flags,
            // 4x MSAA is required for Direct3D Feature Level 10.1 or better
            Some(levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            feature_level,
            context,
        )?;
    }
    // Instance data uses typed R32_UINT buffers, supported by feature level
    // 10.1 without the optional compute/structured-buffer capability.
    Ok(device.unwrap())
}

impl DirectXDevices {
    #[cfg(test)]
    pub(crate) fn software_for_test() -> Result<Self> {
        // Force the original Win7 WARP feature level even on newer Windows.
        let mut context = None;
        let device = get_device_with_levels(
            None,
            Some(&mut context),
            None,
            false,
            &[D3D_FEATURE_LEVEL_10_1],
        )?;
        let dxgi_device: IDXGIDevice = device.cast()?;
        let adapter: IDXGIAdapter1 = unsafe { dxgi_device.GetAdapter() }?.cast()?;
        let dxgi_factory = unsafe { adapter.GetParent::<IDXGIFactory1>() }?;
        Ok(Self {
            adapter,
            dxgi_factory,
            device,
            device_context: context.context("WARP device returned no immediate context")?,
        })
    }
}
