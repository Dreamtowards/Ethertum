use crate::ObjSettings;
use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::mesh::Mesh;
use bevy::reflect::TypePath;
use bevy::tasks::ConditionalSendFuture;

#[derive(Default, TypePath)]
pub struct ObjLoader;

impl AssetLoader for ObjLoader {
    type Error = ObjError;
    type Settings = ObjSettings;
    type Asset = Mesh;

    fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        _: &mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            load_obj_as_mesh(&bytes, settings)
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
    InvalidFile(#[from] wobj::WobjError),
}

pub fn load_obj_as_mesh(bytes: &[u8], settings: &ObjSettings) -> Result<Mesh, ObjError> {
    let obj = wobj::Obj::parse(bytes)?;

    let mut indicies = Vec::new();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    for mesh in obj.meshes() {
        let trimesh = mesh.triangulate()?;
        let start_index = positions.len();
        indicies.extend(trimesh.0.0.into_iter().map(|i| i + start_index));
        positions.extend(trimesh.1.positions);
        if let Some(v) = trimesh.1.normals {
            normals.extend(v);
        }
        if let Some(v) = trimesh.1.uvs {
            uvs.extend(v);
        }
    }

    let len = positions.len();
    Ok(crate::util::to_bevy_mesh(
        wobj::Indicies(indicies),
        wobj::Vertices {
            positions,
            normals: if normals.len() == len { Some(normals) } else { None },
            uvs: if uvs.len() == len { Some(uvs) } else { None },
        },
        settings,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    #[test]
    fn triangle_retains_geometry_and_flips_texture_coordinates() {
        let source = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0.1 0.2\nvt 0.3 0.4\nvt 0.5 0.6\nf 1/1 2/2 3/3\n";
        let mesh = load_obj_as_mesh(source, &ObjSettings::default()).unwrap();
        assert_eq!(mesh.count_vertices(), 3);
        assert_eq!(mesh.indices().unwrap().len(), 3);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {
            panic!("OBJ texture coordinates must be present");
        };
        assert_eq!(uvs[0], [0.1, 0.8]);
    }

    #[test]
    fn invalid_utf8_returns_an_error() {
        assert!(load_obj_as_mesh(&[0xff], &ObjSettings::default()).is_err());
    }
}
