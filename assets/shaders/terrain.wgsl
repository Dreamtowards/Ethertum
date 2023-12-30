
// #import bevy_pbr::{
//     pbr_functions::alpha_discard,
//     pbr_fragment::pbr_input_from_standard_material,
//     forward_io::{VertexOutput, FragmentOutput},
//     pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
// }
#import bevy_pbr::forward_io::VertexOutput

// #import bevy_pbr::mesh_bindings
// #import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_bindings

// #import bevy_pbr::fog
// #import bevy_pbr::shadows
// #import bevy_pbr::lighting
// #import bevy_pbr::pbr_ambient
// #import bevy_pbr::clustered_forward

// #import bevy_pbr::utils

#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_fragment

// struct TerrainMaterial {
//     val: f32,//vec4<f32>,
// };

// @group(1) @binding(0) var<uniform> material: TerrainMaterial;
// @group(1) @binding(1) var base_color_texture: texture_2d<f32>;
// @group(1) @binding(2) var base_color_sampler: sampler;

// struct MyExtendedMaterial {
//     quantize_steps: f32,
// }

// @group(1) @binding(100)
// var<uniform> my_extended_material: MyExtendedMaterial;

@fragment
fn fragment(
    vert_out: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    //let tex = textureSample(base_color_texture, base_color_sampler, vert_out.uv);

 
    // var pbr_in = pbr_fragment::pbr_input_from_vertex_output(vert_out, is_front, false);//pbr_types::pbr_input_new();
    // pbr_in.material.base_color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    var pbr_in = pbr_fragment::pbr_input_from_standard_material(vert_out, is_front);
    
    var color = pbr_functions::apply_pbr_lighting(pbr_in); //pbr_fn::pbr(pbr_in); 

    color = pbr_functions::main_pass_post_lighting_processing(pbr_in, color);
    return color;
    //return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}