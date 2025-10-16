use super::*;

pub struct DDGI2Plugin;

#[derive(Resource, Clone, Copy, Reflect, Debug, ShaderType)]
pub struct DDGI2Uniforms {
    grid_origin: Vec3,
    probe_counts: UVec3,
    probe_spacing: Vec3,
    rays_per_probe: u32,
}

#[derive(Resource)]
struct DDGI2Pipeline {
    pipeline_id: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl Plugin for DDGI2Plugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DDGI2Uniforms::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, prepare_ddgi_pipeline.in_set(bevy::render::RenderSet::Prepare));
    }
}

impl Default for DDGI2Uniforms {
    fn default() -> Self {
         Self {
            grid_origin: Vec3::new(-16.0, 0.0, -16.0),
            probe_counts: UVec3::new(8, 4, 8),
            probe_spacing: Vec3::splat(4.0),
            rays_per_probe: 256,
         }
    }
}

fn prepare_ddgi_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    let bind_group_layout = render_device.create_bind_group_layout(
        "ddgi2_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                binding_types::uniform_buffer::<DDGI2Uniforms>(false),
                binding_types::texture_2d(TextureSampleType::Float { filterable: true }),
                binding_types::texture_2d(TextureSampleType::Float { filterable: true }),
                binding_types::texture_2d(TextureSampleType::Float { filterable: true }),
                binding_types::texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
            ),
        ),
    );

    let shader = asset_server.load("shaders/ddgi_irradiance.wgsl");
    let pipeline_id = pipeline_cache.queue_compute_pipeline(
        ComputePipelineDescriptor {
            label: Some("ddgi2_irradiance_pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: vec![],
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: true,
        },
    );

    commands.insert_resource(DDGI2Pipeline {
        pipeline_id,
        bind_group_layout,
    });
}