use super::*;

pub struct DDGI1Plugin;

#[derive(Resource, Clone, Copy, Reflect, Debug, ShaderType)]
pub struct DDGI1Uniforms {
    grid_origin: Vec3,
    grid_size: IVec3,
    probe_spacing: f32,
    ddgi_irradiance_resolution: u32,
    ddgi_distance_resolution: u32,
}

#[derive(Resource)]
struct DDGISettings {
    enabled: bool,
    num_layers: u32,
    ddgi_uniforms: DDGI1Uniforms,
}

#[derive(Resource)]
struct DDGIProbeGrid {
    probes: Vec<Entity>,
    grid_origin: Vec3,
    grid_size: IVec3,
    probe_spacing: f32,
}

#[derive(Component)]
struct DDGIProbe {
    position: Vec3,
    irradiance_data: Vec<Vec4>,
    distance_data: Vec<f32>,
    needs_update: bool,
}

#[derive(Resource)]
struct DDGIIrradianceTexture {
    image: Handle<Image>,
    size: UVec2,
}

#[derive(Resource)]
struct DDGIDistanceTexture {
    image: Handle<Image>,
    size: UVec2,
}

impl Plugin for DDGI1Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ddgi_system)
            .add_systems(Update, (update_probe_locations, update_irradiance_cache).run_if(condition::in_world))
            .add_systems(Update, update_terrain_material_ddgi_bindings
                .run_if(resource_exists::<DDGIIrradianceTexture>)
                .run_if(resource_exists::<DDGIDistanceTexture>)
            )
            .add_systems(Update, debug_tex.run_if(|cli: Res<ClientInfo>| cli.dbg_tex))
            .insert_resource(DDGISettings::default());
    }
}

impl Default for DDGI1Uniforms {
    fn default() -> Self {
        Self {
            grid_origin: Vec3::new(-32.0, 0.0, -32.0),
            grid_size: IVec3::new(16, 8, 16),
            probe_spacing: 4.0,
            ddgi_irradiance_resolution: 8,
            ddgi_distance_resolution: 16,
        }
    }
}

impl Default for DDGISettings {
    fn default() -> Self {
        Self {
            enabled: true,
            num_layers: 6,
            ddgi_uniforms: DDGI1Uniforms::default(),
        }
    }
}

impl DDGIProbeGrid {
    fn new(
        settings: &DDGISettings,
        grid_origin: Vec3,
    ) -> Self {
        Self {
            probes: Vec::new(),
            grid_origin,
            grid_size: settings.ddgi_uniforms.grid_size,
            probe_spacing: settings.ddgi_uniforms.probe_spacing,
        }
    }

    fn get_probe_position(&self, grid_coord: &IVec3) -> Vec3 {
        self.grid_origin + grid_coord.as_vec3() * self.probe_spacing
    }
}

impl DDGIProbe {
    fn new(
        position: Vec3,
        settings: &DDGISettings,
    ) -> Self {
        Self {
            position,
            irradiance_data: vec![Vec4::ZERO; (settings.ddgi_uniforms.ddgi_irradiance_resolution * settings.ddgi_uniforms.ddgi_irradiance_resolution * settings.num_layers) as usize],
            distance_data: vec![0.0; (settings.ddgi_uniforms.ddgi_distance_resolution * settings.ddgi_uniforms.ddgi_distance_resolution * settings.num_layers) as usize],
            needs_update: true,
        }
    }

    fn update_probe_irradiance(
        &mut self,
        chunk_system: &ClientChunkSystem,
        settings: &DDGISettings,
    ) {
        let irradiance_res = settings.ddgi_uniforms.ddgi_irradiance_resolution as usize;
        let distance_res = settings.ddgi_uniforms.ddgi_distance_resolution as usize;

        for face in 0..settings.num_layers as usize {
            let face_normal = utils::get_cubemap_face_normal(face);
            let face_tangent = utils::get_cubemap_face_tangent(face);
            let face_bitangent = utils::get_cubemap_face_bitangent(face);
            for y in 0..irradiance_res {
                for x in 0..irradiance_res {
                    let u = (x as f32 + 0.5) / irradiance_res as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / irradiance_res as f32 * 2.0 - 1.0;
                    let world_dir = (face_normal + u * face_tangent + v * face_bitangent).normalize();

                    let irradiance = self.trace_irradiance_ray(chunk_system, &world_dir);
                    let distance = self.trace_distance_ray(chunk_system, &world_dir);

                    let irradiance_texel_index = face * irradiance_res * irradiance_res + y * irradiance_res + x;
                    if irradiance_texel_index < self.irradiance_data.len() {
                        self.irradiance_data[irradiance_texel_index] = Vec4::new(irradiance.x, irradiance.y, irradiance.z, 1.0);
                    }

                    let distance_texel_index = face * distance_res * distance_res + y * distance_res + x;
                    if distance_texel_index < self.distance_data.len() {
                        self.distance_data[distance_texel_index] = distance;
                    }
                }
            }
        }
    }

    fn trace_irradiance_ray(
        &self,
        chunk_system: &ClientChunkSystem,
        direction: &Vec3,
    ) -> Vec3 {
        let max_distance = 32.0;
        let step_size = 0.5;
        let num_steps = (max_distance / step_size) as i32;

        let mut current_pos = self.position;
        let mut accumulated_light = Vec3::ZERO;
        let mut total_samples = 0;
        for step in 0..num_steps {
            current_pos += direction * step_size;
            if let Some(voxel) = chunk_system.get_voxel(current_pos.as_ivec3()) {
                if !voxel.is_isoval_empty() {
                    let surface_light = utils::calculate_surface_lighting(chunk_system, current_pos, direction, voxel);
                    accumulated_light += surface_light;
                    break;
                } else {
                    let ambient_light = utils::sample_ambient_lighting(chunk_system, &current_pos);
                    accumulated_light += ambient_light * 0.1;
                    total_samples += 1;
                }
            }

            let distance_factor = 1.0 - (step as f32 * step_size / max_distance);
            if distance_factor <= 0.0 {
                break;
            }
        }

        if total_samples > 0 {
            accumulated_light / total_samples as f32
        } else {
            accumulated_light
        }
    }

    fn trace_distance_ray(
        &self,
        chunk_system: &ClientChunkSystem,
        direction: &Vec3,
    ) -> f32 {
        let max_distance = 64.0;
        let step_size = 0.25;
        let num_steps = (max_distance / step_size) as i32;

        let mut current_pos = self.position;
        for step in 0..num_steps {
            current_pos += direction * step_size;
            if let Some(voxel) = chunk_system.get_voxel(current_pos.as_ivec3()) {
                if !voxel.is_isoval_empty() {
                    return step as f32 * step_size;
                }
            }
        }

        max_distance
    }

    fn update_probe_texture_data(
        &self,
        images: &mut Assets<Image>,
        settings: &DDGISettings,
        irradiance_tex: &DDGIIrradianceTexture,
        distance_tex: &DDGIDistanceTexture,
    ) {
        if let Some(image) = images.get_mut(&irradiance_tex.image) {
            utils::update_irradiance_texture_region(image, &self.irradiance_data, settings.ddgi_uniforms.ddgi_irradiance_resolution);
        }
        if let Some(image) = images.get_mut(&distance_tex.image) {
            utils::update_distance_texture_region(image, &self.distance_data, settings.ddgi_uniforms.ddgi_distance_resolution);
        }
    }
}

fn setup_ddgi_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    settings: Res<DDGISettings>,
) {
    let mut probe_grid = DDGIProbeGrid::new(&settings, Vec3::new(-32.0, 0.0, 32.0));
    for x in 0..probe_grid.grid_size.x {
        for y in 0..probe_grid.grid_size.y {
            for z in 0..probe_grid.grid_size.z {
                let grid_coord = IVec3::new(x, y, z);
                let probe_pos = probe_grid.get_probe_position(&grid_coord);

                let probe = DDGIProbe::new(probe_pos, &settings);
                let entity = commands.spawn((
                    probe,
                    Transform::from_translation(probe_pos),
                    GlobalTransform::default(),
                )).id();
                probe_grid.probes.push(entity);
            }
        }
    }
    commands.insert_resource(probe_grid);

    let (image, size) = utils::create_ddgi_image(&settings, &mut images, true);
    commands.insert_resource(DDGIIrradianceTexture {
        image,
        size,
    });

    let (image, size) = utils::create_ddgi_image(&settings, &mut images, false);
    commands.insert_resource(DDGIDistanceTexture {
        image,
        size,
    });
}

fn debug_tex(
    mut cli: ResMut<ClientInfo>,
    mut images: ResMut<Assets<Image>>,
    ddgi_irradiance_texture: Option<Res<DDGIIrradianceTexture>>,
    ddgi_distance_texture: Option<Res<DDGIDistanceTexture>>,
) {
    cli.dbg_tex = false;
    
    if let (Some(irradiance_tex), Some(distance_tex)) = (ddgi_irradiance_texture, ddgi_distance_texture) {
        if let Some(image) = images.get_mut(&irradiance_tex.image) {
            utils::save_tex(image.data.as_ref().unwrap(), "irradiance_tex.png");
        }
        //utils::save_tex(&distance_tex.image.data, "distance_tex.png");
    }
}

fn update_probe_locations(
    mut probe_query: Query<(&mut DDGIProbe, &Transform)>,
    probe_grid: Res<DDGIProbeGrid>,
) {
    for (mut probe, transform) in probe_query.iter_mut() {
        if probe.position != transform.translation {
            probe.position = transform.translation;
            probe.needs_update = true;
        }
    }
}

fn update_irradiance_cache(
    chunk_system: Res<ClientChunkSystem>,
    settings: Res<DDGISettings>,
    mut images: ResMut<Assets<Image>>,
    mut probe_query: Query<&mut DDGIProbe>,
    ddgi_irradiance_texture: Option<Res<DDGIIrradianceTexture>>,
    ddgi_distance_texture: Option<Res<DDGIDistanceTexture>>,
) {
    if !settings.enabled {
        return;
    }

    if let (Some(irradiance_tex), Some(distance_tex)) = (ddgi_irradiance_texture, ddgi_distance_texture) {
        for mut probe in probe_query.iter_mut() {
            if probe.needs_update {
                probe.needs_update = false;
                
                probe.update_probe_irradiance(&chunk_system, &settings);
                probe.update_probe_texture_data(&mut images, &settings, &irradiance_tex, &distance_tex);
            }
        }
    }
}

fn update_terrain_material_ddgi_bindings(
    chunk_system: Option<Res<ClientChunkSystem>>,
    mut materials: ResMut<Assets<bevy::pbr::ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    ddgi_uniforms: Option<Res<DDGI1Uniforms>>,
    ddgi_irradiance_texture: Option<Res<DDGIIrradianceTexture>>,
    ddgi_distance_texture: Option<Res<DDGIDistanceTexture>>,
) {
    if let (Some(uniforms), Some(irradiance_tex), Some(distance_tex), Some(chunk_sys)) = (ddgi_uniforms, ddgi_irradiance_texture, ddgi_distance_texture, chunk_system) {
        if let Some(material) = materials.get_mut(&chunk_sys.mtl_terrain) {
            material.extension.ddgi_uniforms = uniforms.clone();
            material.extension.ddgi_irradiance_texture = Some(irradiance_tex.image.clone());
            material.extension.ddgi_distance_texture = Some(distance_tex.image.clone());
        }
    }
}

mod utils {
    use super::*;

    pub fn get_cubemap_face_normal(face: usize) -> Vec3 {
        match face {
            0 => Vec3::X,
            1 => Vec3::NEG_X,
            2 => Vec3::Y,
            3 => Vec3::NEG_Y,
            4 => Vec3::Z,
            5 => Vec3::NEG_Z,
            _ => Vec3::Z,
        }
    }

    pub fn get_cubemap_face_tangent(face: usize) -> Vec3 {
        match face {
            0 | 1 => Vec3::NEG_Z,
            2 | 3 => Vec3::X,
            4 | 5 => Vec3::X,
            _ => Vec3::X,
        }
    }

    pub fn get_cubemap_face_bitangent(face: usize) -> Vec3 {
        match face {
            0 => Vec3::NEG_Y,
            1 => Vec3::Y,
            2 => Vec3::Z,
            3 => Vec3::NEG_Z,
            4 => Vec3::NEG_Y,
            5 => Vec3::Y,
            _ => Vec3::Y,
        }
    }
    
    pub fn create_ddgi_image(
        settings: &DDGISettings,
        images: &mut Assets<Image>,
        is_irradiance: bool,
    ) -> (Handle<Image>, UVec2) {
        let (resolution, texture_dimension, texture_format) = if is_irradiance {
            (settings.ddgi_uniforms.ddgi_irradiance_resolution, TextureDimension::D3, TextureFormat::Rgba16Float)
        } else {
            (settings.ddgi_uniforms.ddgi_distance_resolution, TextureDimension::D2, TextureFormat::Rg16Float)
        };

        let texture_size = UVec3::new(
            (settings.ddgi_uniforms.grid_size.x * settings.ddgi_uniforms.grid_size.z) as u32 * resolution,
            settings.ddgi_uniforms.grid_size.y as u32 * resolution,
            settings.num_layers,
        );
        let pixels_data = vec![0_u8; (texture_size.x * texture_size.y * texture_size.z) as usize];

        let mut image = Image::new_fill(
            Extent3d {
                width: texture_size.x,
                height: texture_size.y,
                depth_or_array_layers: texture_size.z,
            },
            texture_dimension,
            &pixels_data,
            texture_format,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

        (images.add(image), texture_size.truncate())
    }

    fn get_voxel_density(
        chunk_system: &ClientChunkSystem,
        position: Vec3,
    ) -> f32 {
        if let Some(voxel) = chunk_system.get_voxel(position.as_ivec3()) {
            if voxel.is_isoval_empty() {
                0.0
            } else {
                1.0
            }
        } else {
            0.0
        }
    }

    fn estimate_surface_normal(
        chunk_system: &ClientChunkSystem,
        position: Vec3,
    ) -> Vec3 {
        let epsilon = 0.1;
        let mut normal = Vec3::ZERO;
        for axis in 0..3 {
            let mut pos_offset = position;
            let mut neg_offset = position;

            pos_offset[axis] += epsilon;
            neg_offset[axis] -= epsilon;

            let pos_density = get_voxel_density(chunk_system, pos_offset);
            let neg_density = get_voxel_density(chunk_system, neg_offset);

            normal[axis] = pos_density - neg_density;
        }

        normal.normalize_or_zero()
    }

    fn get_voxel_color(tex_id: u16) -> Vec3 {
        match tex_id {
            1 => Vec3::new(0.5, 0.3, 0.2),
            2 => Vec3::new(0.2, 0.8, 0.2),
            3 => Vec3::new(0.6, 0.6, 0.6),
            _ => Vec3::new(0.5, 0.5, 0.5),
        }
    }

    pub fn calculate_surface_lighting(
        chunk_system: &ClientChunkSystem,
        position: Vec3,
        ray_direction: &Vec3,
        voxel: &crate::voxel::Vox,
    ) -> Vec3 {
        let normal = estimate_surface_normal(chunk_system, position);
        let voxel_light = voxel.light.red() as f32 / MAX_LIGHT_LEVEL as f32;
        let base_color = get_voxel_color(voxel.tex_id);

        let ndotl = (-ray_direction).dot(normal).max(0.0);
        let diffuse = base_color * voxel_light * ndotl;
        let ambient = base_color * 0.2;

        diffuse + ambient
    }

    pub fn sample_ambient_lighting(
        chunk_system: &ClientChunkSystem,
        position: &Vec3,
    ) -> Vec3 {
        let mut total_light = 0.0;
        let mut sample_count = 0;

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let sample_pos = position + Vec3::new(dx as f32, dy as f32, dz as f32);
                    if let Some(voxel) = chunk_system.get_voxel(sample_pos.as_ivec3()) {
                        total_light += voxel.light.red() as f32;
                        sample_count += 1;
                    }
                }
            }
        }

        if sample_count > 0 {
            let avg_light = total_light / (sample_count as f32 * MAX_LIGHT_LEVEL as f32);
            Vec3::splat(avg_light)
        } else {
            Vec3::splat(0.1)
        }
    }

    pub fn update_irradiance_texture_region(
        image: &mut Image,
        data: &[Vec4],
        resolution: u32,
    ) {
        let region_size = (resolution * resolution * 6) as usize;
        let bytes_per_pixel = 8;

        if data.len() >= region_size {
            if let Some(image_data) = &mut image.data {
                for (i, texel) in data.iter().take(region_size).enumerate() {
                    let byte_offset = i * bytes_per_pixel;
                    if byte_offset + bytes_per_pixel <= image_data.len() {
                        let (r, g, b, a) = (half::f16::from_f32(texel.x), half::f16::from_f32(texel.y), half::f16::from_f32(texel.z), half::f16::from_f32(texel.w));
                        let bytes = [
                            r.to_le_bytes()[0], r.to_le_bytes()[1],
                            g.to_le_bytes()[0], g.to_le_bytes()[1],
                            b.to_le_bytes()[0], b.to_le_bytes()[1],
                            a.to_le_bytes()[0], a.to_le_bytes()[1],
                        ];
                        image_data[byte_offset..byte_offset + 8].copy_from_slice(&bytes);
                    }
                }
            }
        }
    }

    pub fn update_distance_texture_region(
        image: &mut Image,
        data: &[f32],
        resolution: u32,
    ) {
        let region_size = (resolution * resolution * 6) as usize;
        let bytes_per_pixel = 4;

        if data.len() >= region_size {
            if let Some(image_data) = &mut image.data {
                for (i, &distance) in data.iter().take(region_size).enumerate() {
                    let byte_offset = i * bytes_per_pixel;
                    if byte_offset + bytes_per_pixel <= image_data.len() {
                        let r = half::f16::from_f32(distance);
                        let g = half::f16::from_f32(distance * distance);

                        let bytes = [
                            r.to_le_bytes()[0], r.to_le_bytes()[1],
                            g.to_le_bytes()[0], g.to_le_bytes()[1],
                        ];
                        image_data[byte_offset..byte_offset + bytes_per_pixel].copy_from_slice(&bytes);
                    }
                }
            }
        }
    }
    
    pub fn save_tex(
        image_data: &[u8],
        path: &str,
    ) {
        let s = format!("{}/{}", std::env::var("HOME").unwrap(), path);
        let output_path = std::path::Path::new(&s);
        image::save_buffer(output_path, image_data, 32, 32, image::ColorType::Rgba8).unwrap();
    }
}