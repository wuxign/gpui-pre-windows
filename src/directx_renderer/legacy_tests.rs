use super::{DXGI_SWAP_EFFECT_DISCARD, DirectXRenderer};
use crate::DirectXDevices;
use gpui::{
    AtlasKey, Bounds, ContentMask, DevicePixels, FontId, GlyphId, MonochromeSprite, Path,
    PlatformAtlas, PolychromeSprite, Quad, RenderGlyphParams, ScaledPixels, Scene, SubpixelSprite,
    TransformationMatrix, Underline, WindowBackgroundAppearance, point, px, rgb, size,
};
use windows::{Win32::UI::WindowsAndMessaging::*, core::w};

#[test]
fn legacy_graphics_draws_and_resizes_discard_swap_chain() {
    // Run with GPUI_WIN7_COMPAT=1 so this also exercises the old path on modern Windows.
    if !crate::compat::legacy_mode() {
        return;
    }
    draw_and_resize(DirectXDevices::new().unwrap());
}

#[test]
fn software_graphics_draws_and_resizes_discard_swap_chain() {
    if !crate::compat::legacy_mode() {
        return;
    }
    draw_and_resize(DirectXDevices::software_for_test().unwrap());
}

fn draw_and_resize(devices: DirectXDevices) {
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            w!("STATIC"),
            w!("GPUI graphics test"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            128,
            128,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    };
    struct Window(windows::Win32::Foundation::HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }
    let _window = Window(hwnd);
    let mut renderer = DirectXRenderer::new(hwnd, &devices, true).unwrap();
    for (width, height) in [(64, 48), (96, 72), (48, 36), (128, 96), (64, 48)] {
        renderer
            .resize(size(DevicePixels(width), DevicePixels(height)))
            .unwrap();
        let description =
            unsafe { renderer.resources.as_ref().unwrap().swap_chain.GetDesc() }.unwrap();
        assert_eq!(description.SwapEffect, DXGI_SWAP_EFFECT_DISCARD);
        assert_eq!(
            (description.BufferDesc.Width, description.BufferDesc.Height),
            (width as u32, height as u32)
        );
        let bounds = Bounds::new(
            point(ScaledPixels(0.), ScaledPixels(0.)),
            size(ScaledPixels(width as f32), ScaledPixels(height as f32)),
        );
        let mut scene = Scene::default();
        scene.insert_primitive(Quad {
            bounds,
            content_mask: ContentMask { bounds },
            background: rgb(0xff0000).into(),
            ..Default::default()
        });
        let rect = |x, y, w, h| {
            Bounds::new(
                point(ScaledPixels(x), ScaledPixels(y)),
                size(ScaledPixels(w), ScaledPixels(h)),
            )
        };
        // Overlap to force separate batches with nonzero starting indices.
        for (bounds, color) in [
            (rect(2., 2., 8., 8.), 0x00ff00),
            (rect(5., 5., 2., 2.), 0x0000ff),
        ] {
            scene.insert_primitive(Quad {
                bounds,
                content_mask: ContentMask { bounds },
                background: rgb(color).into(),
                ..Default::default()
            });
        }
        scene.insert_primitive(Underline {
            order: 0,
            pad: 0,
            bounds: rect(18., 2., 8., 3.),
            content_mask: ContentMask { bounds },
            color: rgb(0xffffff).into(),
            thickness: ScaledPixels(3.),
            wavy: false.into(),
        });
        let mut path = Path::new(point(px(2.), px(18.)));
        path.line_to(point(px(10.), px(18.)));
        path.line_to(point(px(2.), px(26.)));
        path.color = rgb(0xffff00).into();
        path.content_mask = ContentMask {
            bounds: bounds.map(|p| px(p.0)),
        };
        scene.insert_primitive(path.scale(1.));
        let mut tiles = Vec::new();
        for (is_emoji, subpixel_rendering, channels) in
            [(false, false, 1), (false, true, 4), (true, false, 4)]
        {
            let key = AtlasKey::Glyph(RenderGlyphParams {
                font_id: FontId(0),
                glyph_id: GlyphId(0),
                font_size: px(4.),
                subpixel_variant: point(0, 0),
                scale_factor: 1.,
                is_emoji,
                subpixel_rendering,
                dilation: 0,
            });
            let tile = renderer
                .atlas
                .get_or_insert_with(key, &mut || {
                    Ok(Some((
                        size(DevicePixels(4), DevicePixels(4)),
                        std::borrow::Cow::Owned(vec![255; 16 * channels]),
                    )))
                })
                .unwrap()
                .unwrap();
            tiles.push(tile);
        }
        scene.insert_primitive(MonochromeSprite {
            order: 0,
            pad: 0,
            bounds: rect(0., 0., 4., 4.),
            content_mask: ContentMask { bounds },
            color: rgb(0xffffff).into(),
            tile: tiles[0],
            // Non-symmetric matrix checks row/column layout as well.
            transformation: TransformationMatrix {
                rotation_scale: [[1., 0.5], [0., 1.]],
                translation: [18., 18.],
            },
        });
        scene.insert_primitive(SubpixelSprite {
            order: 0,
            pad: 0,
            bounds: rect(28., 18., 4., 4.),
            content_mask: ContentMask { bounds },
            color: rgb(0).into(),
            tile: tiles[1],
            transformation: TransformationMatrix::unit(),
        });
        scene.insert_primitive(PolychromeSprite {
            order: 0,
            pad: 0,
            grayscale: false.into(),
            opacity: 1.,
            bounds: rect(36., 18., 4., 4.),
            content_mask: ContentMask { bounds },
            corner_radii: Default::default(),
            tile: tiles[2],
        });
        scene.finish();
        renderer
            .draw(&scene, WindowBackgroundAppearance::Opaque)
            .unwrap();
        let image = renderer
            .render_to_image(&scene, WindowBackgroundAppearance::Opaque)
            .unwrap();
        assert_eq!(image.dimensions(), (width as u32, height as u32));
        for (x, y, expected) in [
            (3, 3, [0, 255, 0, 255]),
            (5, 5, [0, 0, 255, 255]),
            (20, 3, [255, 255, 255, 255]),
            (3, 19, [255, 255, 0, 255]),
            (22, 21, [255, 255, 255, 255]),
            (29, 19, [0, 0, 0, 255]),
            (37, 19, [255, 255, 255, 255]),
        ] {
            assert_eq!(image.get_pixel(x, y).0, expected, "pixel ({x}, {y})");
        }
        assert_eq!(
            image.get_pixel((width - 1) as u32, (height - 1) as u32).0,
            [255, 0, 0, 255]
        );
    }
}
