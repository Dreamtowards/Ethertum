
// #import bevy_pbr::{
//     pbr_functions::alpha_discard,
//     pbr_fragment::pbr_input_from_standard_material,
//     forward_io::{VertexOutput, FragmentOutput},
//     pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
// }
// #import bevy_pbr::mesh_bindings
// #import bevy_pbr::mesh_view_bindings
// #import bevy_pbr::pbr_bindings
// #import bevy_pbr::fog
// #import bevy_pbr::shadows
// #import bevy_pbr::lighting
// #import bevy_pbr::pbr_ambient
// #import bevy_pbr::clustered_forward
// #import bevy_pbr::utils

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::forward_io::Vertex
#import bevy_pbr::mesh_functions

#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_fragment

struct TerrainMaterial {
    val: f32,//vec4<f32>,
};

struct MyVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,

    @location(2) bary: vec3<f32>,
    @location(3) mtls: vec3<f32>,  // material texture ids. u32

    @location(5) @interpolate(flat) instance_index: u32,
}

@vertex
fn vertex(
    @builtin(vertex_index) vtx_index: u32,
    in: Vertex,
) -> MyVertexOutput {

    let model = mesh_functions::get_model_matrix(in.instance_index);
    let inst_idx = bevy_render::instance_index::get_instance_index(in.instance_index);

    var out: MyVertexOutput;
    out.position = mesh_functions::mesh_position_local_to_clip(model, vec4<f32>(in.position, 1.0));
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(in.position, 1.0));
    out.world_normal = mesh_functions::mesh_normal_local_to_world(in.normal, inst_idx); 
    out.instance_index = inst_idx;

    let vi = vtx_index % 3u;
    let bary = vec3<f32>(f32(vi == 0u), f32(vi == 1u), f32(vi == 2u));
    out.bary = bary;
    out.mtls = bary * vec3<f32>(in.uv.x, in.uv.x, in.uv.x);

    return out;
}


@group(1) @binding(0) var<uniform> material: TerrainMaterial;
@group(1) @binding(1) var tex_diffuse: texture_2d<f32>;
@group(1) @binding(2) var _sampler: sampler;

fn _mod(v: f32, n: f32) -> f32 {
    let f = v % n;
    return select(f, f + n, f < 0.0);
}

// Texture Triplanar Mapping
fn tex_trip(
    tex: texture_2d<f32>,
    mtl_id: f32,
    p: vec3<f32>,
    blend: vec3<f32>,
) -> vec4<f32> {

    let num_mtls = 24.0;
    let tex_mul_x = 1.0 / num_mtls;
    let tex_add_x = mtl_id / num_mtls;
    let uvX = vec2<f32>(tex_add_x + _mod(-p.z * tex_mul_x, tex_mul_x), 1.0-p.y);
    let uvY = vec2<f32>(tex_add_x + _mod( p.x * tex_mul_x, tex_mul_x),     p.z);
    let uvZ = vec2<f32>(tex_add_x + _mod( p.x * tex_mul_x, tex_mul_x), 1.0-p.y);

    return 
        textureSample(tex, _sampler, fract(uvX)) * blend.x + 
        textureSample(tex, _sampler, fract(uvY)) * blend.y + 
        textureSample(tex, _sampler, fract(uvZ)) * blend.z;
}

@fragment
fn fragment(
    in: MyVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    let worldpos  = in.world_position.xyz;
    let worldnorm = in.world_normal;
    let mtls = in.mtls / in.bary;

    let tex = textureSample(tex_diffuse, _sampler, fract(worldpos.xz));

    var blend_trip = abs(worldnorm);
    blend_trip /= blend_trip.x + blend_trip.y + blend_trip.z;  // makesure sum = 1
    
    var vert_out: VertexOutput;
    vert_out.position = in.position;
    vert_out.world_position = in.world_position;
    vert_out.world_normal = in.world_normal;
    vert_out.instance_index = in.instance_index;
    var pbr_in = pbr_fragment::pbr_input_from_vertex_output(vert_out, is_front, false);

    pbr_in.material.base_color = tex_trip(tex_diffuse, 1.0, worldpos, blend_trip);//vec4<f32>(in.bary, 1.0);
    
    var color = pbr_functions::apply_pbr_lighting(pbr_in);
    color = pbr_functions::main_pass_post_lighting_processing(pbr_in, color);
    return color;
}