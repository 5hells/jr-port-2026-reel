struct Globals {
    params: vec4<f32>, // x: time, y: transition, z: res_x, w: res_y
    color_a_old: vec4<f32>,
    color_a_new: vec4<f32>,
    color_b_old: vec4<f32>,
    color_b_new: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4<f32>(pos[idx], 0.0, 1.0);
}


@fragment
fn fs_main(@builtin(position) clip_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let time = globals.params.x;
    let mix_progress = globals.params.y; // 0.0 to 1.0
    let res = globals.params.zw;
    
    let uv = clip_pos.xy / res;
    let p = (uv - 0.5) * vec2<f32>(res.x / res.y, 1.0);

    // --- STEP 1: Temporal Smoothing ---
    // This smoothly interpolates the static colors over time
    let current_a = mix(globals.color_a_old.rgb, globals.color_a_new.rgb, mix_progress);
    let current_b = mix(globals.color_b_old.rgb, globals.color_b_new.rgb, mix_progress);

    // --- STEP 2: Spatial Mesh Logic ---
    let p1 = vec2<f32>(sin(time * 0.4), cos(time * 0.3)) * 0.5;
    let p2 = vec2<f32>(cos(time * 0.5), sin(time * 0.4)) * 0.5;

    let w1 = 1.0 / (pow(length(p - p1), 2.0) + 0.2);
    let w2 = 1.0 / (pow(length(p - p2), 2.0) + 0.2);
    
    // Normalize weights
    let n1 = w1 / (w1 + w2);
    let n2 = w2 / (w1 + w2);

    // Blend the temporally-smoothed colors spatially
    var final_rgb = (current_a * n1) + (current_b * n2);

    // Ribbons and Noise... (ps3-style, beautiful ribbon-like waves, horizontally-stretched, in vertical center), slow and evolving/shifting over time
    // ref: let ribbon = smoothstep(0.02, 0.0, abs(uv.y - wave)) * 0.2;
    let ribbon = smoothstep(0.02, 0.0, abs(uv.y - (0.5 + 0.05 * sin(time + uv.x * 10.0)))) * 0.2;
    let ribbon_2 = smoothstep(0.02, 0.0, abs(uv.y - (0.5 + 0.05 * cos(time * 1.5 + uv.x * 12.0)))) * 0.2;
    let ribbon_3 = smoothstep(0.02, 0.0, abs(uv.y - (0.5 + 0.05 * sin(time * 0.8 + uv.x * 8.0)))) * 0.2;
    let ribbon_4 = smoothstep(0.02, 0.0, abs(uv.y - (0.5 + 0.05 * cos(time * 1.2 + uv.x * 15.0)))) * 0.2;

    return vec4<f32>(final_rgb + vec3<f32>(ribbon + ribbon_2 + ribbon_3 + ribbon_4), 1.0);
}