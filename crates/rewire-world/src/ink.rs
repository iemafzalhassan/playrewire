//! Ink look: one fullscreen pass after the main 3D pass.
//!
//! It reads three things: the lit-free colour (meshes use unlit materials), the view
//! depth texture and the normal prepass. From those it does
//! 1. flat 2-band toon shading (world normal against a fixed light direction), and
//! 2. an edge-detect outline from depth and normal discontinuities, drawn in ink.
//!
//! WebGL2 cannot copy depth textures, so `DepthPrepass` is unavailable there. We read
//! the main view depth texture directly instead (bound as an unfilterable float
//! texture, `Msaa::Off`), the same trick Bevy's motion blur uses.

use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::{
        FullscreenShader,
        prepass::ViewPrepassTextures,
        schedule::{Core3d, Core3dSystems},
        tonemapping::tonemapping,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FilterMode, FragmentState,
            Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
            TextureSampleType,
            binding_types::{sampler, texture_2d, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{ExtractedView, ViewDepthStencilTexture, ViewTarget},
    },
    shader::Shader,
};

/// Ink `#2a2620` from the design tokens, in linear space for the shader.
pub fn ink_color() -> LinearRgba {
    Color::srgb_u8(0x2a, 0x26, 0x20).to_linear()
}

/// Put this on a `Camera3d` (with `NormalPrepass`, `Msaa::Off`, and a depth texture
/// that has `TEXTURE_BINDING`) to get the ink look.
#[derive(Component, Clone, Copy, ShaderType, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
#[extract_app(RenderApp)]
pub struct InkOutline {
    /// Outline colour (linear RGBA).
    pub ink: Vec4,
    /// xyz: direction *towards* the light (world space). w: shade multiplier for the dark band.
    pub light: Vec4,
    /// x: relative depth threshold, y: normal dot threshold, z: sample offset in px, w: unused.
    pub params: Vec4,
}

impl Default for InkOutline {
    fn default() -> Self {
        Self {
            ink: ink_color().to_vec4(),
            light: Vec3::new(0.45, 0.8, 0.35).normalize().extend(0.72),
            params: Vec4::new(0.04, 0.75, 2.0, 0.0),
        }
    }
}

pub struct InkPlugin;

impl Plugin for InkPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "ink.wgsl");
        app.add_plugins((
            ExtractComponentPlugin::<InkOutline>::default(),
            UniformComponentPlugin::<InkOutline>::default(),
        ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<InkPipeline>>()
            .add_systems(RenderStartup, init_ink_pipeline)
            .add_systems(Render, prepare_ink_pipelines.in_set(RenderSystems::Prepare))
            .add_systems(
                Core3d,
                ink_pass
                    .in_set(Core3dSystems::PostProcess)
                    .before(tonemapping),
            );
    }
}

#[derive(Resource)]
pub struct InkPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

fn init_ink_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "ink_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // Colour from the main pass.
                texture_2d(TextureSampleType::Float { filterable: false }),
                // Normal prepass (world normal * 0.5 + 0.5).
                texture_2d(TextureSampleType::Float { filterable: false }),
                // View depth (reverse-Z), read as a plain float texture.
                texture_2d(TextureSampleType::Float { filterable: false }),
                // Nearest sampler. WebGL2 needs a sampler paired with each texture: an
                // unpaired texelFetch reads an incomplete texture and returns zeros.
                sampler(SamplerBindingType::NonFiltering),
                uniform_buffer::<InkOutline>(true),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Nearest,
        ..default()
    });
    commands.insert_resource(InkPipeline {
        layout,
        sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "ink.wgsl"),
    });
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct InkPipelineKey {
    target_format: TextureFormat,
}

impl SpecializedRenderPipeline for InkPipeline {
    type Key = InkPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("ink_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

#[derive(Component)]
pub struct InkPipelineId(CachedRenderPipelineId);

fn prepare_ink_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<InkPipeline>>,
    pipeline: Res<InkPipeline>,
    views: Query<(Entity, &ExtractedView), With<InkOutline>>,
) {
    for (entity, view) in &views {
        let id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            InkPipelineKey {
                target_format: view.target_format,
            },
        );
        commands.entity(entity).insert(InkPipelineId(id));
    }
}

fn ink_pass(
    view: ViewQuery<(
        &ViewTarget,
        &ViewPrepassTextures,
        &ViewDepthStencilTexture,
        &DynamicUniformIndex<InkOutline>,
        &InkPipelineId,
    )>,
    ink_pipeline: Res<InkPipeline>,
    pipeline_cache: Res<PipelineCache>,
    uniforms: Res<ComponentUniforms<InkOutline>>,
    mut ctx: RenderContext,
) {
    let (view_target, prepass, depth, uniform_index, pipeline_id) = view.into_inner();
    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
        return;
    };
    let Some(normal_view) = prepass.normal_view() else {
        return;
    };
    let Some(depth_view) = depth.attachment.depth_stencil_views().depth_only_view() else {
        return;
    };
    let Some(uniform_binding) = uniforms.uniforms().binding() else {
        return;
    };

    let post_process = view_target.post_process_write();
    let bind_group = ctx.render_device().create_bind_group(
        Some("ink_bind_group"),
        &pipeline_cache.get_bind_group_layout(&ink_pipeline.layout),
        &BindGroupEntries::sequential((
            post_process.source,
            normal_view,
            depth_view,
            &ink_pipeline.sampler,
            uniform_binding.clone(),
        )),
    );

    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("ink_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[uniform_index.index()]);
    pass.draw(0..3, 0..1);
}
