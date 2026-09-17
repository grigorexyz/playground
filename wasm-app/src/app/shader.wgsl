// Instanced quad shader shared by every rectangle the desktop draws
// (window chrome, taskbar, and per-pixel glyphs). Four vertices per
// instance are emitted as a triangle strip covering `rect`.

struct Globals {
    resolution: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    let corner = vec2<f32>(
        f32(vertex_index & 1u),
        f32((vertex_index >> 1u) & 1u),
    );
    let pixel_pos = rect.xy + corner * rect.zw;
    let clip_x = (pixel_pos.x / globals.resolution.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (pixel_pos.y / globals.resolution.y) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
