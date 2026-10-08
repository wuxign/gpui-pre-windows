use crate::{
    DirectXDevices, direct_write::ModernDirectWriteTextSystem,
    legacy_direct_write::LegacyDirectWriteTextSystem,
};
use anyhow::Result;
use gpui::*;
use std::borrow::Cow;

pub(crate) enum DirectWriteTextSystem {
    Modern(ModernDirectWriteTextSystem),
    Legacy(LegacyDirectWriteTextSystem),
}

macro_rules! dispatch {
    ($this:expr, $method:ident $(, $arg:expr)*) => {
        match $this {
            DirectWriteTextSystem::Modern(system) => system.$method($($arg),*),
            DirectWriteTextSystem::Legacy(system) => system.$method($($arg),*),
        }
    };
}

impl DirectWriteTextSystem {
    pub(crate) fn new(devices: Option<&DirectXDevices>) -> Result<Self> {
        if !crate::compat::legacy_mode() {
            match ModernDirectWriteTextSystem::new(devices) {
                Ok(system) => return Ok(Self::Modern(system)),
                Err(error) => log::warn!(
                    "Modern DirectWrite unavailable: {error:#}; trying the legacy backend"
                ),
            }
        }
        log::info!("Using the experimental Windows 7 DirectWrite backend");
        Ok(Self::Legacy(LegacyDirectWriteTextSystem::new(devices)?))
    }

    pub(crate) fn handle_gpu_lost(&self, devices: &DirectXDevices) -> Result<()> {
        dispatch!(self, handle_gpu_lost, devices)
    }
    pub(crate) fn release_gpu(&self) {
        dispatch!(self, release_gpu)
    }
}

impl PlatformTextSystem for DirectWriteTextSystem {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        dispatch!(self, add_fonts, fonts)
    }
    fn all_font_names(&self) -> Vec<String> {
        dispatch!(self, all_font_names)
    }
    fn font_id(&self, font: &Font) -> Result<FontId> {
        dispatch!(self, font_id, font)
    }
    fn font_metrics(&self, id: FontId) -> FontMetrics {
        dispatch!(self, font_metrics, id)
    }
    fn typographic_bounds(&self, font: FontId, glyph: GlyphId) -> Result<Bounds<f32>> {
        dispatch!(self, typographic_bounds, font, glyph)
    }
    fn advance(&self, font: FontId, glyph: GlyphId) -> Result<Size<f32>> {
        dispatch!(self, advance, font, glyph)
    }
    fn glyph_for_char(&self, font: FontId, ch: char) -> Option<GlyphId> {
        dispatch!(self, glyph_for_char, font, ch)
    }
    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        dispatch!(self, glyph_raster_bounds, params)
    }
    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        dispatch!(self, rasterize_glyph, params, bounds)
    }
    fn layout_line(&self, text: &str, size: Pixels, runs: &[FontRun]) -> LineLayout {
        dispatch!(self, layout_line, text, size, runs)
    }
    fn recommended_rendering_mode(&self, font: FontId, size: Pixels) -> TextRenderingMode {
        dispatch!(self, recommended_rendering_mode, font, size)
    }
}
