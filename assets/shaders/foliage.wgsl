

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::forward_io::Vertex


@vertex
fn vertex(
    in: Vertex,
) -> VertexOutput {

    let model = mesh_functions::get_model_matrix(in.instance_index);
    let inst_idx = bevy_render::instance_index::get_instance_index(in.instance_index);

    var out: VertexOutput;
    out.position = mesh_functions::mesh_position_local_to_clip(model, vec4<f32>(in.position, 1.0));
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(in.position, 1.0));
    out.world_normal = mesh_functions::mesh_normal_local_to_world(in.normal, inst_idx); 
    out.instance_index = inst_idx;

    return out;
}



@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    let worldpos  = in.world_position.xyz;
    let worldnorm = in.world_normal;

    return ;
}