//! Instance-buffer compatibility for original Win7 WARP.
use super::*;

// Validate the instance ABI used by legacy_buffers.hlsl. No source generation.
const _: () = {
    use std::mem::{offset_of, size_of};
    assert!(size_of::<Quad>() == 160);
    assert!(offset_of!(Quad, background) == 40);
    assert!(offset_of!(Quad, border_widths) == 144);
    assert!(size_of::<Shadow>() == 112);
    assert!(offset_of!(Shadow, inset) == 104);
    assert!(size_of::<Underline>() == 64);
    assert!(offset_of!(Underline, wavy) == 60);
    assert!(size_of::<MonochromeSprite>() == 112);
    assert!(size_of::<SubpixelSprite>() == 112);
    assert!(offset_of!(MonochromeSprite, transformation) == 88);
    assert!(offset_of!(SubpixelSprite, transformation) == 88);
    assert!(size_of::<PolychromeSprite>() == 96);
    assert!(offset_of!(PolychromeSprite, tile) == 64);
    assert!(size_of::<PathRasterizationSprite>() == 104);
    assert!(offset_of!(PathRasterizationSprite, bounds) == 88);
    assert!(size_of::<PathSprite>() == 16);
    assert!(size_of::<Background>() == 72);
    assert!(size_of::<AtlasTile>() == 32);
    assert!(offset_of!(AtlasTile, bounds) == 16);
};

pub(super) fn font_info() -> FontInfo {
    unsafe {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
            .expect("DirectWrite factory unavailable");
        let params = factory
            .CreateRenderingParams()
            .expect("DirectWrite rendering parameters unavailable");
        let grayscale_contrast = params
            .cast::<IDWriteRenderingParams1>()
            .ok()
            .map(|params| params.GetGrayscaleEnhancedContrast())
            .unwrap_or_else(|| params.GetEnhancedContrast());
        FontInfo {
            gamma_ratios: gpui::get_gamma_correction_ratios(params.GetGamma()),
            grayscale_enhanced_contrast: grayscale_contrast,
            subpixel_enhanced_contrast: params.GetEnhancedContrast(),
            is_bgr: params.GetPixelGeometry() == DWRITE_PIXEL_GEOMETRY_BGR,
        }
    }
}

pub(super) fn create_legacy_swap_chain(
    factory: &IDXGIFactory1,
    device: &ID3D11Device,
    hwnd: HWND,
    width: u32,
    height: u32,
) -> Result<IDXGISwapChain> {
    let desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: width,
            Height: height,
            Format: RENDER_TARGET_FORMAT,
            ..Default::default()
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: BUFFER_COUNT as u32,
        OutputWindow: hwnd,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        Flags: 0,
    };
    let mut swap_chain = None;
    unsafe {
        factory
            .CreateSwapChain(device, &desc, &mut swap_chain)
            .ok()?
    };
    swap_chain.context("Missing legacy swap chain")
}

#[inline]
pub(super) fn create_buffer(
    device: &ID3D11Device,
    element_size: usize,
    buffer_size: usize,
) -> Result<ID3D11Buffer> {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: (element_size * buffer_size) as u32,
        Usage: D3D11_USAGE_DYNAMIC,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
        MiscFlags: 0,
        StructureByteStride: 0,
    };
    let mut buffer = None;
    unsafe { device.CreateBuffer(&desc, None, Some(&mut buffer)) }?;
    Ok(buffer.unwrap())
}

#[inline]
pub(super) fn create_buffer_view(
    device: &ID3D11Device,
    buffer: &ID3D11Buffer,
) -> Result<Option<ID3D11ShaderResourceView>> {
    let mut view = None;
    let mut buffer_desc = D3D11_BUFFER_DESC::default();
    unsafe { buffer.GetDesc(&mut buffer_desc) };
    let desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
        Format: DXGI_FORMAT_R32_UINT,
        ViewDimension: D3D_SRV_DIMENSION_BUFFER,
        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
            Buffer: D3D11_BUFFER_SRV {
                Anonymous1: D3D11_BUFFER_SRV_0 { FirstElement: 0 },
                Anonymous2: D3D11_BUFFER_SRV_1 {
                    NumElements: buffer_desc.ByteWidth / 4,
                },
            },
        },
    };
    unsafe { device.CreateShaderResourceView(buffer, Some(&desc), Some(&mut view)) }?;
    Ok(view)
}
