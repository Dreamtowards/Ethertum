use bevy::{asset::RenderAssetUsages, prelude::*};

/// Empty chunk meshes must not enter Bevy 0.19.1's GPU slab upload path.
/// The caller also hides their entities, since an earlier GPU mesh can remain cached.
pub(super) fn prepare_chunk_mesh(mesh: &mut Mesh) -> Visibility {
    if mesh.count_vertices() == 0 {
        mesh.asset_usage = RenderAssetUsages::MAIN_WORLD;
        Visibility::Hidden
    } else {
        mesh.asset_usage = RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD;
        Visibility::Visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::vtx::VertexBuffer;
    use bevy::mesh::PrimitiveTopology;

    #[test]
    fn empty_populated_empty_mesh_updates_control_upload_and_visibility() {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        let mut buffer = VertexBuffer::default();
        buffer.to_mesh(&mut mesh);
        assert_eq!(prepare_chunk_mesh(&mut mesh), Visibility::Hidden);
        assert!(!mesh.asset_usage.contains(RenderAssetUsages::RENDER_WORLD));

        for pos in [Vec3::ZERO, Vec3::X, Vec3::Z] {
            buffer.push_vertex(pos, Vec2::ZERO, Vec3::Y);
        }
        buffer.to_mesh(&mut mesh);
        assert_eq!(prepare_chunk_mesh(&mut mesh), Visibility::Visible);
        assert!(mesh.asset_usage.contains(RenderAssetUsages::RENDER_WORLD));

        buffer.clear();
        buffer.to_mesh(&mut mesh);
        assert_eq!(prepare_chunk_mesh(&mut mesh), Visibility::Hidden);
        assert!(!mesh.asset_usage.contains(RenderAssetUsages::RENDER_WORLD));
    }
}
