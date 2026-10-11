use crate::ObjSettings;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

pub(crate) fn to_bevy_mesh(
    indicies: wobj::Indicies,
    verticies: wobj::Vertices,
    settings: &ObjSettings,
) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    mesh.insert_indices(Indices::U32(indices(indicies)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verticies.positions);

    if let Some(mut uvs) = verticies.uvs {
        convert_uvs(&mut uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }

    if let Some(normals) = verticies.normals
        && !settings.force_compute_normals
    {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    } else if settings.prefer_flat_normals {
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();
    } else {
        mesh.compute_normals();
    }

    mesh
}

fn indices(indicies: wobj::Indicies) -> Vec<u32> {
    indicies.0.into_iter().map(|i| i as u32).collect()
}

fn convert_uvs(uvs: &mut Vec<[f32; 2]>) {
    for uv in uvs {
        *uv = [uv[0], 1.0 - uv[1]];
    }
}
