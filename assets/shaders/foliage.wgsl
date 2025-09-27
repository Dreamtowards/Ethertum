
#import bevy_pbr::prepass_io::Vertex
#import bevy_pbr::prepass_io::VertexOutput
#import bevy_pbr::prepass_io::FragmentOutput
// #import bevy_pbr::forward_io::Vertex
// #import bevy_pbr::forward_io::VertexOutput

#import bevy_pbr::mesh_functions
#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_functions
#import bevy_pbr::pbr_fragment

#import bevy_render::globals::Globals
@group(0) @binding(1) var<uniform> globals: Globals;

// #import bevy_render::globals::Globals
// @group(0) @binding(1) var<uniform> globals: Globals;
// let time = globals.time;

// @group(2) @binding(0) var _sampler: sampler;
// @group(2) @binding(1) var tex_diffuse: texture_2d<f32>;

// @group(2) @binding(2) var<uniform> time: f32;


@vertex
fn vertex(
    in: Vertex,
) -> VertexOutput {

    let inst_idx = in.instance_index;
    let model = mesh_functions::get_world_from_local(inst_idx);

    var time = globals.time;
    var localpos = in.position;
    var wave_speed = 0.8;
    var wave_span = 1000.0;
    var wave_amplitude = 0.16;
    localpos += cos(localpos.yzx * wave_span + time * wave_speed) * wave_amplitude;

    let norm = vec3<f32>(0, 1.0, 0); //normalize(in.normal + vec3<f32>(0, 10.0, 0));

    var out: VertexOutput;
    out.position = mesh_functions::mesh_position_local_to_clip(model, vec4<f32>(localpos, 1.0));
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(localpos, 1.0));
    out.world_normal = norm; //mesh_functions::mesh_normal_local_to_world(in.normal, inst_idx); 
    out.uv = in.uv;
    out.instance_index = inst_idx;

    return out;
}



@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // let worldpos  = in.world_position.xyz;
    // var worldnorm = in.world_normal;//select(-in.world_normal, in.world_normal, is_front);
    // worldnorm = normalize(worldnorm + vec3<f32>(0, 20.1, 0));

    let base_color = textureSample(bevy_pbr::pbr_bindings::base_color_texture, bevy_pbr::pbr_bindings::base_color_sampler, in.uv);

    if base_color.a < 0.5 {
        discard;
    }

    var pbr_in = pbr_fragment::pbr_input_from_vertex_output(in, /*is_front*/is_front, /*double_sided*/true);
    pbr_in.N = vec3<f32>(0, 1, 0);
    pbr_in.material.base_color = base_color;
    pbr_in.material.perceptual_roughness = 1.0;
    pbr_in.material.reflectance = vec3<f32>(0.0);
    pbr_in.material.specular_transmission = 0.0;
    // pbr_in.material.ior = 0.0;
    // pbr_in.material.metallic = select(0.0, 0.9, (mtls[vi_mtl]) == 9. || (mtls[vi_mtl]) == 8.);
    // pbr_in.diffuse_occlusion = vec3<f32>(1.0);
    // pbr_in.specular_occlusion = 0.0;

    // var color = pbr_functions::apply_pbr_lighting(pbr_in);
    // pbr_in.material.flags |= pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;  // enable fog
    // color = pbr_functions::main_pass_post_lighting_processing(pbr_in, color);
    // // color = vec4<f32>((worldnorm+1.0)/2.0, 1.0); 
    // // color = vec4<f32>(worldnorm, 1.0);
    // return color;

    return bevy_pbr::pbr_deferred_functions::deferred_output(in, pbr_in);
}