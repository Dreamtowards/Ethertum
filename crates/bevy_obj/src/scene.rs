use std::path::{Path, PathBuf};

use crate::ObjSettings;
use crate::util::to_bevy_mesh;
use bevy::asset::AssetPath;
use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::tasks::ConditionalSendFuture;

#[derive(Component, Reflect, Default, Debug, Deref, DerefMut)]
#[reflect(Component)]
pub struct MeshGroups(pub Vec<String>);

#[derive(Default, TypePath)]
pub struct ObjLoader;

impl AssetLoader for ObjLoader {
    type Error = ObjError;
    type Settings = ObjSettings;
    type Asset = WorldAsset;

    fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            load_obj_as_scene(&bytes, load_context, settings).await
        })
    }

    fn extensions(&self) -> &[&str] {
        crate::EXTENSIONS
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ObjError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid OBJ file: {0}")]
    ObjParseError(wobj::WobjError),
    #[error("Invalid MTL file: {0}")]
    MtlParseError(wobj::WobjError),
    #[error("Failed to read MTL file: {0}")]
    MtlReadError(#[from] bevy::asset::ReadAssetBytesError),
    #[error("Material '{0}' not found in '{1}'")]
    MaterialNotFound(String, PathBuf),
    #[error("Invalid mesh: {0}")]
    InvalidMesh(wobj::WobjError),
    #[error("Failed to resolve path: {0}")]
    ResolveError(#[from] bevy::asset::ParseAssetPathError),
}

fn resolve_path<'a>(source: &AssetPath<'a>, path: &Path) -> Result<AssetPath<'static>, ObjError> {
    if path.is_relative()
        && let Some(path) = path.to_str()
        && let Some(parent) = source.parent()
    {
        Ok(parent.resolve_str(path)?)
    } else {
        Ok(path.to_path_buf().into())
    }
}

struct MtlCache {
    cache: HashMap<PathBuf, wobj::Mtl>,
}

impl MtlCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::with_capacity(1),
        }
    }

    pub async fn load(
        &mut self,
        ctx: &mut LoadContext<'_>,
        path: &PathBuf,
        name: &str,
    ) -> Result<&wobj::Material, ObjError> {
        if !self.cache.contains_key(path) {
            let asset_path = resolve_path(ctx.path(), path)?;
            let bytes = ctx.read_asset_bytes(asset_path).await?;
            let mtl = wobj::Mtl::parse(&bytes).map_err(ObjError::MtlParseError)?;
            self.cache.insert(path.clone(), mtl);
        }

        self.cache
            .get(path)
            .and_then(|mtl| mtl.get(name))
            .ok_or_else(|| ObjError::MaterialNotFound(name.to_string(), path.to_path_buf()))
    }
}

async fn load_materials(
    ctx: &mut LoadContext<'_>,
    materials: HashSet<Option<(PathBuf, String)>>,
) -> Result<HashMap<Option<(PathBuf, String)>, Handle<StandardMaterial>>, ObjError> {
    let mut handles = HashMap::new();
    let mut mtls = MtlCache::new();

    fn default_material(ctx: &mut LoadContext<'_>) -> Handle<StandardMaterial> {
        const DEFAULT_MATERIAL_LABEL: &str = "Material.Default";
        if ctx.has_labeled_asset(DEFAULT_MATERIAL_LABEL) {
            ctx.get_label_handle(DEFAULT_MATERIAL_LABEL)
        } else {
            ctx.add_labeled_asset(
                DEFAULT_MATERIAL_LABEL.to_string(),
                StandardMaterial::default(),
            )
        }
    }

    for (i, mat_key) in materials.into_iter().enumerate() {
        let handle = if let Some((path, name)) = &mat_key {
            match mtls.load(ctx, path, name).await {
                Ok(mtl_mat) => {
                    let material = convert_material(ctx, mtl_mat)?;
                    let label = format!("Material{i}");
                    Some(ctx.add_labeled_asset(label, material))
                }
                Err(error) => {
                    error!("Failed to load MTL material: {error}");
                    None
                }
            }
        } else {
            None
        };

        handles.insert(mat_key, handle.unwrap_or_else(|| default_material(ctx)));
    }

    Ok(handles)
}

fn load_texture(ctx: &mut LoadContext, path: &Path) -> Result<Handle<Image>, ObjError> {
    Ok(ctx.load(resolve_path(ctx.path(), path)?))
}

fn convert_material(
    ctx: &mut LoadContext<'_>,
    material: &wobj::Material,
) -> Result<StandardMaterial, ObjError> {
    let mut m = StandardMaterial::default();

    if let Some(v) = &material.diffuse {
        match *v {
            wobj::ColorValue::RGB(r, g, b) => m.base_color = Color::srgb(r, g, b),
            wobj::ColorValue::XYZ(x, y, z) => m.base_color = Color::xyz(x, y, z),
            _ => (),
        }
    }
    if let Some(v) = &material.specular {
        match *v {
            wobj::ColorValue::RGB(r, g, b) => m.specular_tint = Color::srgb(r, g, b),
            wobj::ColorValue::XYZ(x, y, z) => m.specular_tint = Color::xyz(x, y, z),
            _ => (),
        }
    }
    if let Some(v) = &material.emissive {
        match *v {
            wobj::ColorValue::RGB(r, g, b) => m.emissive = LinearRgba::rgb(r, g, b),
            wobj::ColorValue::XYZ(x, y, z) => m.emissive = Color::xyz(x, y, z).into(),
            _ => (),
        }
    }

    fn apply_f32(input: Option<f32>, target: &mut f32) {
        *target = input.unwrap_or(*target)
    }

    apply_f32(material.roughness, &mut m.perceptual_roughness);
    apply_f32(material.metallic, &mut m.metallic);
    apply_f32(material.cc_thickness, &mut m.clearcoat);
    apply_f32(material.cc_roughness, &mut m.clearcoat_perceptual_roughness);
    apply_f32(material.anisotropy, &mut m.anisotropy_strength);
    apply_f32(material.anisotropy_rotation, &mut m.anisotropy_rotation);

    if let Some(map) = &material.diffuse_map {
        m.base_color_texture = Some(load_texture(ctx, map.path())?)
    }

    // Treat both normal and bump map as normal map texture
    if let Some(map) = material.normal_map.as_ref().or(material.bump_map.as_ref()) {
        m.normal_map_texture = Some(load_texture(ctx, map.path())?)
    }

    // Enable alpha blend if the material has a dissolve map
    if material.dissolve_map.is_some() {
        m.alpha_mode = AlphaMode::Blend
    }

    Ok(m)
}

async fn load_obj_as_scene<'a>(
    bytes: &'a [u8],
    ctx: &'a mut LoadContext<'_>,
    settings: &'a ObjSettings,
) -> Result<WorldAsset, ObjError> {
    let obj = wobj::Obj::parse(bytes).map_err(ObjError::ObjParseError)?;

    let mut materials = HashSet::new();
    let mut meshes = Vec::new();

    for obj_mesh in obj.meshes() {
        let material = obj_mesh
            .mtllib()
            .map(PathBuf::from)
            .zip(obj_mesh.material().map(String::from));
        materials.insert(material.clone());

        let (indicies, vertices) = obj_mesh.triangulate().map_err(ObjError::InvalidMesh)?;
        let name = obj_mesh.name().map(str::to_owned);
        let groups = obj_mesh.groups().to_vec();
        meshes.push((name, groups, indicies, vertices, material));
    }

    let mat_handles = load_materials(ctx, materials).await?;

    let mut world = World::default();
    for (i, (name, groups, indicies, verticies, mat_key)) in meshes.into_iter().enumerate() {
        let mesh_handle = ctx.add_labeled_asset(
            format!("Mesh{i}"),
            to_bevy_mesh(indicies, verticies, settings),
        );

        let mut entity = world.spawn((
            Name::new(name.unwrap_or_else(|| format!("Mesh{i}"))),
            Mesh3d(mesh_handle),
            MeshMaterial3d(mat_handles[&mat_key].clone()),
        ));

        if !groups.is_empty() {
            entity.insert(MeshGroups(groups));
        }
    }

    Ok(WorldAsset::new(world))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path() {
        let source = AssetPath::from("models/cube.obj");
        assert_eq!(
            resolve_path(&source, &PathBuf::from("cube.mtl")).unwrap(),
            AssetPath::from("models/cube.mtl")
        );
        assert_eq!(
            resolve_path(&source, &PathBuf::from("subdir/cube.mtl")).unwrap(),
            AssetPath::from("models/subdir/cube.mtl")
        );
        assert_eq!(
            resolve_path(&source, &PathBuf::from("/absolute/cube.mtl")).unwrap(),
            AssetPath::from("/absolute/cube.mtl")
        );

        let source = AssetPath::from("/models/cube.obj");
        assert_eq!(
            resolve_path(&source, &PathBuf::from("cube.mtl")).unwrap(),
            AssetPath::from("/models/cube.mtl")
        );
        assert_eq!(
            resolve_path(&source, &PathBuf::from("subdir/cube.mtl")).unwrap(),
            AssetPath::from("/models/subdir/cube.mtl")
        );
        assert_eq!(
            resolve_path(&source, &PathBuf::from("/absolute/cube.mtl")).unwrap(),
            AssetPath::from("/absolute/cube.mtl")
        );

        let source = AssetPath::from("https://example.com/models/cube.obj");
        assert_eq!(
            resolve_path(&source, &PathBuf::from("cube.mtl")).unwrap(),
            AssetPath::from("https://example.com/models/cube.mtl")
        );
        assert_eq!(
            resolve_path(&source, &PathBuf::from("subdir/cube.mtl")).unwrap(),
            AssetPath::from("https://example.com/models/subdir/cube.mtl")
        );
        assert_eq!(
            resolve_path(&source, &PathBuf::from("/absolute/cube.mtl")).unwrap(),
            AssetPath::from("/absolute/cube.mtl")
        );
    }
}
