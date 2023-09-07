@group(0) @binding(1) var fx_texture: texture_2d<f32>;
@group(1) @binding(1) var frame_texture: texture_2d<f32>;
@group(1) @binding(0) var out_texture: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(8, 8, 1)
fn additive(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let pos = global_invocation_id.xy;
    let size = vec2<u32>(textureDimensions(fx_texture));

    if any(size < pos) {
        return;
    }

    let frame_color = textureLoad(frame_texture, pos, 0).rgb;
    let fx_color = textureLoad(fx_texture, pos, 0).rgb;

    let result = frame_color + fx_color;

    textureStore(out_texture, pos, vec4<f32>(result, 1.0));
}