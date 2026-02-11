struct Globals {
    params: vec4<f32>, // x: time, y: res_x, z: res_y, w: unused
};

@group(0) @binding(0) var<uniform> globals: Globals;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4<f32>(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) clip_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let time = globals.params.x * 0.5; // Slow down time for smoothness
    let res = globals.params.yz;
    let uv = clip_pos.xy / res;
    
    // Center coordinates and fix aspect ratio
    var p = (uv - 0.5) * vec2<f32>(res.x / res.y, 1.0);
    
    // --- Domain Warping ---
    // We displace the coordinate system over and over to create "liquid" flow
    for (var i: f32 = 1.0; i < 4.0; i += 1.0) {
        p.x += 0.3 / i * sin(i * 3.0 * p.y + time + i * 0.5);
        p.y += 0.3 / i * cos(i * 3.0 * p.x + time + i * 0.5);
    }

    // --- Psychedelic Color Mapping ---
    // Using a cosine-based palette for a vibrant rainbow effect
    let r = 0.5 + 0.5 * sin(time + p.x + 0.0);
    let g = 0.5 + 0.5 * sin(time + p.y + 2.0);
    let b = 0.5 + 0.5 * sin(time + p.x + p.y + 4.0);
    
    // Add a "shimmer" layer based on the length of the warped vector
    let brightness = 0.2 / length(p);
    var final_color = vec3<f32>(r, g, b) + (brightness * 0.3);

    // Final contrast adjustment
    final_color = smoothstep(vec3(0.0), vec3(1.0), final_color);

    return vec4<f32>(final_color, 1.0);
}