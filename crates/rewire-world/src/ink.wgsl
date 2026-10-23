// Ink look: flat 2-band toon shading with pen hatching in the shade, and a wobbly,
// gently "boiling" screen-space outline from depth and normal discontinuities.
// Self-contained WGSL (no imports) so it loads the same on WebGL2 and native.

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct InkOutline {
    // Outline colour, linear RGBA.
    ink: vec4<f32>,
    // xyz: direction towards the light (world). w: multiplier for the shaded band.
    light: vec4<f32>,
    // x: depth crease threshold, y: normal dot threshold, z: sample offset (px),
    // w: hatch spacing (px).
    params: vec4<f32>,
    // x: line wobble amplitude (px), y: wobble frequency (1/px), z: boil seed,
    // w: paper grain strength.
    wobble: vec4<f32>,
};

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var normal_texture: texture_2d<f32>;
@group(0) @binding(2) var depth_texture: texture_2d<f32>;
@group(0) @binding(3) var nearest: sampler;
@group(0) @binding(4) var<uniform> ink: InkOutline;

// Sampling (not textureLoad) on purpose: on WebGL2 a texture read without a paired
// sampler is incomplete and returns zeros.
fn fetch(t: texture_2d<f32>, p: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
    let q = clamp(p, vec2<i32>(0), dims - vec2<i32>(1));
    let uv = (vec2<f32>(q) + vec2<f32>(0.5)) / vec2<f32>(dims);
    return textureSampleLevel(t, nearest, uv, 0.0);
}

fn depth_at(p: vec2<i32>, dims: vec2<i32>) -> f32 {
    return fetch(depth_texture, p, dims).x;
}

fn normal_at(p: vec2<i32>, dims: vec2<i32>) -> vec3<f32> {
    return fetch(normal_texture, p, dims).xyz * 2.0 - vec3<f32>(1.0);
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

// Smooth value noise in [0, 1].
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Is there a line at q? Silhouettes against the paper, creases in depth, and folds in
// the surface normal. The depth test uses the second difference (d(+r) + d(-r) - 2 d):
// reverse-Z depth is affine across any flat face, so flat faces never fill with ink,
// however steeply they are seen, and a road painted flat on the ground gets no line.
fn is_edge(q: vec2<i32>, dims: vec2<i32>, r: i32) -> bool {
    let d0 = depth_at(q, dims);
    let bg0 = d0 <= 0.0;
    let n0 = normal_at(q, dims);
    var axes = array<vec2<i32>, 2>(vec2<i32>(r, 0), vec2<i32>(0, r));
    for (var k = 0; k < 2; k++) {
        let a = q + axes[k];
        let b = q - axes[k];
        let da = depth_at(a, dims);
        let db = depth_at(b, dims);
        if (da <= 0.0) != bg0 || (db <= 0.0) != bg0 {
            return true;
        }
        if bg0 {
            continue;
        }
        if abs(da + db - 2.0 * d0) / d0 > ink.params.x {
            return true;
        }
        if dot(n0, normal_at(a, dims)) < ink.params.y || dot(n0, normal_at(b, dims)) < ink.params.y {
            return true;
        }
    }
    return false;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(screen_texture));
    let pf = in.position.xy;
    let p = vec2<i32>(pf);
    let base = fetch(screen_texture, p, dims);
    let d0 = depth_at(p, dims);

    // A hand's wobble: look for edges a pixel or two off, along a slow noise field
    // that is re-seeded every boil step.
    let seed = vec2<f32>(ink.wobble.z * 7.13, ink.wobble.z * 3.71);
    let wp = pf * ink.wobble.y + seed;
    let w = vec2<f32>(noise(wp), noise(wp + vec2<f32>(17.3, 9.1))) - vec2<f32>(0.5);
    let q = p + vec2<i32>(round(w * 2.0 * ink.wobble.x));
    let r = max(i32(ink.params.z), 1);

    if is_edge(q, dims, r) {
        return vec4<f32>(ink.ink.rgb, 1.0);
    }
    if d0 <= 0.0 {
        // Keep the cleared, fully transparent pixel so the CSS paper shows through.
        return base;
    }

    let n = normalize(normal_at(p, dims));
    let lit = dot(n, ink.light.xyz) > 0.2;
    var rgb = base.rgb;
    if !lit {
        // Shade, crossed with pen hatching.
        let spacing = max(ink.params.w, 3.0);
        let h = fract((pf.x + pf.y) / spacing);
        let hatch = select(1.0, 0.82, h < 0.22);
        rgb = rgb * ink.light.w * hatch;
    }
    // Paper grain, so flat colour feels like gouache on paper.
    let grain = noise(pf * 0.45) - 0.5;
    rgb = rgb * (1.0 + grain * ink.wobble.w);
    return vec4<f32>(rgb, base.a);
}
