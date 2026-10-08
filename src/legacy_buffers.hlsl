// Original Win7 WARP uses ordinary typed buffers instead of StructuredBuffer.
// This file is ordinary HLSL, compiled directly in debug and release builds.
// Include one section after its upstream primitive struct is declared.
#ifndef GPUI_LEGACY_BUFFER_COMMON
#define GPUI_LEGACY_BUFFER_COMMON
// Typed buffers are part of Shader Model 4.1, including original Win7 WARP.
// Read the repr(C) instance bytes as 32-bit words, preserving integer fields
// and float bit patterns. Offsets/strides are checked against Rust layouts.
Buffer<uint> instance_words: register(t1);

Bounds load_Bounds(uint offset) {
    Bounds value;
    value.origin = asfloat(uint2(instance_words[offset + 0u], instance_words[offset + 1u]));
    value.size = asfloat(uint2(instance_words[offset + 2u], instance_words[offset + 3u]));
    return value;
}

Corners load_Corners(uint offset) {
    Corners value;
    value.top_left = asfloat(instance_words[offset + 0u]);
    value.top_right = asfloat(instance_words[offset + 1u]);
    value.bottom_right = asfloat(instance_words[offset + 2u]);
    value.bottom_left = asfloat(instance_words[offset + 3u]);
    return value;
}

Edges load_Edges(uint offset) {
    Edges value;
    value.top = asfloat(instance_words[offset + 0u]);
    value.right = asfloat(instance_words[offset + 1u]);
    value.bottom = asfloat(instance_words[offset + 2u]);
    value.left = asfloat(instance_words[offset + 3u]);
    return value;
}

Hsla load_Hsla(uint offset) {
    Hsla value;
    value.h = asfloat(instance_words[offset + 0u]);
    value.s = asfloat(instance_words[offset + 1u]);
    value.l = asfloat(instance_words[offset + 2u]);
    value.a = asfloat(instance_words[offset + 3u]);
    return value;
}

LinearColorStop load_LinearColorStop(uint offset) {
    LinearColorStop value;
    value.color = load_Hsla(offset + 0u);
    value.percentage = asfloat(instance_words[offset + 4u]);
    return value;
}

Background load_Background(uint offset) {
    Background value;
    value.tag = instance_words[offset + 0u];
    value.color_space = instance_words[offset + 1u];
    value.solid = load_Hsla(offset + 2u);
    value.gradient_angle_or_pattern_height = asfloat(instance_words[offset + 6u]);
    value.colors[0] = load_LinearColorStop(offset + 7u);
    value.colors[1] = load_LinearColorStop(offset + 12u);
    value.pad = instance_words[offset + 17u];
    return value;
}

AtlasTextureId load_AtlasTextureId(uint offset) {
    AtlasTextureId value;
    value.index = instance_words[offset + 0u];
    value.kind = instance_words[offset + 1u];
    return value;
}

AtlasBounds load_AtlasBounds(uint offset) {
    AtlasBounds value;
    value.origin = asint(uint2(instance_words[offset + 0u], instance_words[offset + 1u]));
    value.size = asint(uint2(instance_words[offset + 2u], instance_words[offset + 3u]));
    return value;
}

AtlasTile load_AtlasTile(uint offset) {
    AtlasTile value;
    value.texture_id = load_AtlasTextureId(offset + 0u);
    value.tile_id = instance_words[offset + 2u];
    value.padding = instance_words[offset + 3u];
    value.bounds = load_AtlasBounds(offset + 4u);
    return value;
}

TransformationMatrix load_TransformationMatrix(uint offset) {
    TransformationMatrix value;
    value.rotation_scale = transpose(float2x2(asfloat(uint4(instance_words[offset + 0u], instance_words[offset + 1u], instance_words[offset + 2u], instance_words[offset + 3u]))));
    value.translation = asfloat(uint2(instance_words[offset + 4u], instance_words[offset + 5u]));
    return value;
}

#endif

#ifdef GPUI_BUFFER_QUADS
Quad load_Quad(uint offset) {
    Quad value;
    value.order = instance_words[offset + 0u];
    value.border_style = instance_words[offset + 1u];
    value.bounds = load_Bounds(offset + 2u);
    value.content_mask = load_Bounds(offset + 6u);
    value.background = load_Background(offset + 10u);
    value.border_color = load_Hsla(offset + 28u);
    value.corner_radii = load_Corners(offset + 32u);
    value.border_widths = load_Edges(offset + 36u);
    return value;
}

Quad read_quads(uint index) {
    return load_Quad(index * 40u);
}
#endif

#ifdef GPUI_BUFFER_SHADOWS
Shadow load_Shadow(uint offset) {
    Shadow value;
    value.order = instance_words[offset + 0u];
    value.blur_radius = asfloat(instance_words[offset + 1u]);
    value.bounds = load_Bounds(offset + 2u);
    value.corner_radii = load_Corners(offset + 6u);
    value.content_mask = load_Bounds(offset + 10u);
    value.color = load_Hsla(offset + 14u);
    value.element_bounds = load_Bounds(offset + 18u);
    value.element_corner_radii = load_Corners(offset + 22u);
    value.inset = instance_words[offset + 26u];
    value.pad = instance_words[offset + 27u];
    return value;
}

Shadow read_shadows(uint index) {
    return load_Shadow(index * 28u);
}
#endif

#ifdef GPUI_BUFFER_PATH_RASTERIZATION_SPRITES
PathRasterizationSprite load_PathRasterizationSprite(uint offset) {
    PathRasterizationSprite value;
    value.xy_position = asfloat(uint2(instance_words[offset + 0u], instance_words[offset + 1u]));
    value.st_position = asfloat(uint2(instance_words[offset + 2u], instance_words[offset + 3u]));
    value.color = load_Background(offset + 4u);
    value.bounds = load_Bounds(offset + 22u);
    return value;
}

PathRasterizationSprite read_path_rasterization_sprites(uint index) {
    return load_PathRasterizationSprite(index * 26u);
}
#endif

#ifdef GPUI_BUFFER_PATH_SPRITES
PathSprite load_PathSprite(uint offset) {
    PathSprite value;
    value.bounds = load_Bounds(offset + 0u);
    return value;
}

PathSprite read_path_sprites(uint index) {
    return load_PathSprite(index * 4u);
}
#endif

#ifdef GPUI_BUFFER_UNDERLINES
Underline load_Underline(uint offset) {
    Underline value;
    value.order = instance_words[offset + 0u];
    value.pad = instance_words[offset + 1u];
    value.bounds = load_Bounds(offset + 2u);
    value.content_mask = load_Bounds(offset + 6u);
    value.color = load_Hsla(offset + 10u);
    value.thickness = asfloat(instance_words[offset + 14u]);
    value.wavy = instance_words[offset + 15u];
    return value;
}

Underline read_underlines(uint index) {
    return load_Underline(index * 16u);
}
#endif

#ifdef GPUI_BUFFER_MONO_SPRITES
MonochromeSprite load_MonochromeSprite(uint offset) {
    MonochromeSprite value;
    value.order = instance_words[offset + 0u];
    value.pad = instance_words[offset + 1u];
    value.bounds = load_Bounds(offset + 2u);
    value.content_mask = load_Bounds(offset + 6u);
    value.color = load_Hsla(offset + 10u);
    value.tile = load_AtlasTile(offset + 14u);
    value.transformation = load_TransformationMatrix(offset + 22u);
    return value;
}

MonochromeSprite read_mono_sprites(uint index) {
    return load_MonochromeSprite(index * 28u);
}
#endif

#ifdef GPUI_BUFFER_POLY_SPRITES
PolychromeSprite load_PolychromeSprite(uint offset) {
    PolychromeSprite value;
    value.order = instance_words[offset + 0u];
    value.pad = instance_words[offset + 1u];
    value.grayscale = instance_words[offset + 2u];
    value.opacity = asfloat(instance_words[offset + 3u]);
    value.bounds = load_Bounds(offset + 4u);
    value.content_mask = load_Bounds(offset + 8u);
    value.corner_radii = load_Corners(offset + 12u);
    value.tile = load_AtlasTile(offset + 16u);
    return value;
}

PolychromeSprite read_poly_sprites(uint index) {
    return load_PolychromeSprite(index * 24u);
}
#endif
