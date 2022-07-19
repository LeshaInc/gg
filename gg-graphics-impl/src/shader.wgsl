struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) tex_id: u32,
    @location(2) color: vec4<f32>,
};

@group(0) @binding(0)
var textures: binding_array<texture_2d<f32>>;

@group(0) @binding(1)
var linear_sampler: sampler;

@vertex
fn vs_main(
    @location(0) pos: vec2<f32>,
    @location(1) tex: vec2<f32>,
    @location(2) tex_id: u32,
    @location(3) color: vec4<f32>,
) -> VertexOutput {
    var vertex: VertexOutput;
    vertex.pos = vec4<f32>(pos, 0.0, 1.0);
    vertex.tex = tex;
    vertex.tex_id = tex_id;
    vertex.color = color;
    return vertex;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let col = vertex.color;

    let tex = textures[vertex.tex_id];
    let tex_col = textureSample(tex, linear_sampler, vertex.tex);

    let glyph_factor = f32(col.r > 1.5);
    let glyph_color = vec4<f32>(col.r - 2.0, col.g, col.b, tex_col.r);

    return mix(col * tex_col, glyph_color, glyph_factor);
}
