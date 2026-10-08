use anyhow::{Context, Result};
use itertools::Itertools;
use windows::Win32::Graphics::{
    Direct3D::{D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1},
    Direct3D11::{ID3D11Device, ID3D11DeviceContext},
    Dxgi::{IDXGIAdapter1, IDXGIFactory1},
};

#[path = "directx_devices/compat.rs"]
mod device_factory;
use device_factory::{check_debug_layer_available, get_adapter, get_dxgi_factory};

pub(crate) fn try_to_recover_from_device_lost<T>(mut f: impl FnMut() -> Result<T>) -> Result<T> {
    (0..5)
        .map(|i| {
            if i > 0 {
                // Add a small delay before retrying
                std::thread::sleep(std::time::Duration::from_millis(100 + i * 10));
            }
            f()
        })
        .find_or_last(Result::is_ok)
        .unwrap()
        .context("DirectXRenderer failed to recover from lost device after multiple attempts")
}

#[derive(Clone)]
pub(crate) struct DirectXDevices {
    pub(crate) adapter: IDXGIAdapter1,
    pub(crate) dxgi_factory: IDXGIFactory1,
    pub(crate) device: ID3D11Device,
    pub(crate) device_context: ID3D11DeviceContext,
}

impl DirectXDevices {
    pub(crate) fn new() -> Result<Self> {
        let debug_layer_available = check_debug_layer_available();
        let dxgi_factory =
            get_dxgi_factory(debug_layer_available).context("Creating DXGI factory")?;
        let (adapter, device, device_context, feature_level) =
            get_adapter(&dxgi_factory, debug_layer_available).context("Getting DXGI adapter")?;
        // Software devices own an adapter too. Use its factory for swap-chain creation.
        let dxgi_factory = unsafe { adapter.GetParent::<IDXGIFactory1>() }
            .context("Getting device's DXGI factory")?;
        match feature_level {
            D3D_FEATURE_LEVEL_11_1 => {
                log::info!("Created device with Direct3D 11.1 feature level.")
            }
            D3D_FEATURE_LEVEL_11_0 => {
                log::info!("Created device with Direct3D 11.0 feature level.")
            }
            D3D_FEATURE_LEVEL_10_1 => {
                log::info!("Created device with Direct3D 10.1 feature level.")
            }
            _ => unreachable!(),
        }

        Ok(Self {
            adapter,
            dxgi_factory,
            device,
            device_context,
        })
    }
}
