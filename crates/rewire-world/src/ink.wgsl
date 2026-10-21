// Ink look: flat 2-band toon shading + screen-space edge-detect outline.
// Self-contained WGSL (no imports) so it loads the same on WebGL2 and native.

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct InkOutline {
    // Outline colour, linear RGBA.
    ink: vec4<f32>,
    // xyz: direction towards the light (world). w: multiplier for the dark band.
    light: vec4<f32>,
    // x: relative depth threshold, y: normal dot threshold, z: sample offset (px).
    params: vec4<f32>,
};

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var normal_texture: texture_2d<f32>;
@group(0) @binding(2) var depth_texture: texture_2d<f32>;
@group(0) @binding(3) var nearest: sampler;
@group(0) @binding(4) var<uniform> ink: InkOutline;

// Sampling (not textureLoad) on purpose: on WebGL2 a texture read without a paired
// sampler is incomplete and returns zeros.
fn fetch(t: texture_2d<f32>, p: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
    let uv = (vec2<f32>(p) + vec2<f32>(0.5)) / vec2<f32>(dims);
    return textureSampleLevel(t, nearest, uv, 0.0);
}

fn load_normal(p: vec2<i32>, dims: vec2<i32>) -> vec3<f32> {
    return fetch(normal_texture, p, dims).xyz * 2.0 - vec3<f32>(1.0);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(screen_texture));
    let max_p = dims - vec2<i32>(1);
    let p = vec2<i32>(in.position.xy);
    let base = fetch(screen_texture, p, dims);
    // Reverse-Z: 0 is the far plane, so 0 means "nothing drawn here" (paper shows).
    let d0 = fetch(depth_texture, p, dims).x;
    let n0 = load_normal(p, dims);

    let r = i32(ink.params.z);
    var offsets = array<vec2<i32>, 4>(
        vec2<i32>(r, 0),
        vec2<i32>(-r, 0),
        vec2<i32>(0, r),
        vec2<i32>(0, -r),
    );

    var edge = false;
    for (var i = 0; i < 4; i++) {
        let q = clamp(p + offsets[i], vec2<i32>(0), max_p);
        let d1 = fetch(depth_texture, q, dims).x;
        let bg0 = d0 <= 0.0;
        let bg1 = d1 <= 0.0;
        if bg0 != bg1 {
            // Silhouette against the paper.
            edge = true;
        } else if !bg0 {
            // Linear view depth is proportional to 1/d, so z1/z0 = d0/d1.
            if abs(d0 / d1 - 1.0) > ink.params.x {
                edge = true;
            }
            if dot(n0, load_normal(q, dims)) < ink.params.y {
                edge = true;
            }
        }
    }

    if edge {
        return vec4<f32>(ink.ink.rgb, 1.0);
    }
    if d0 <= 0.0 {
        // Keep the cleared, fully transparent pixel so the CSS paper shows through.
        return base;
    }
    let lit = dot(normalize(n0), ink.light.xyz) > 0.2;
    let shade = select(ink.light.w, 1.0, lit);
    return vec4<f32>(base.rgb * shade, base.a);
}
