
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


@group(1) @binding(0) var _sampler: sampler;
@group(1) @binding(1) var tex_diffuse: texture_2d<f32>;
@group(1) @binding(2) var tex_normal: texture_2d<f32>;
@group(1) @binding(3) var tex_dram: texture_2d<f32>;

@group(1) @binding(4) var<uniform> sample_scale: f32;
@group(1) @binding(5) var<uniform> normal_intensity: f32;
@group(1) @binding(6) var<uniform> triplanar_blend_pow: f32;
@group(1) @binding(7) var<uniform> heightmap_blend_pow: f32;


fn _mod(v: f32, n: f32) -> f32 {
    let f = v % n;
    return select(f, f + n, f < 0.0);
}

fn _vec3_max_idx(v: vec3<f32>) -> i32 {
    // if v.x > v.y { if v.x > v.z {0} else {2}} else { if v.y > v.z {1} else {2}}
    return select(select(2, 1, v.y > v.z), select(2, 0, v.x > v.z), v.x > v.y);
}

fn triplanar_uv(mtl_id: f32, _p: vec3<f32>) -> array<vec2<f32>, 3> {
    let num_mtls = 24.0;
	let bias = 0.001 / num_mtls;  // intoduce Epsilon to fix Mipmap Error (and Float-point Error) on Tex Boundary 0.02
    let tex_mul_x = 1.0 / num_mtls;
    let tex_add_x = mtl_id / num_mtls;
    let p = _p / sample_scale;
    let uvX = vec2<f32>(tex_add_x + _mod(-p.z * tex_mul_x, tex_mul_x -bias*2.0) +bias, 1.0-p.y);
    let uvY = vec2<f32>(tex_add_x + _mod( p.x * tex_mul_x, tex_mul_x -bias*2.0) +bias,     p.z);
    let uvZ = vec2<f32>(tex_add_x + _mod( p.x * tex_mul_x, tex_mul_x -bias*2.0) +bias, 1.0-p.y);
    return array(fract(uvX), fract(uvY), fract(uvZ));
}

// Texture Triplanar Mapping
fn triplanar_sample(
    tex: texture_2d<f32>,
    mtl_id: f32,
    p: vec3<f32>,
    blend: vec3<f32>,
) -> vec4<f32> {
    let uvs = triplanar_uv(mtl_id, p);

    return 
        textureSample(tex, _sampler, uvs[0]) * blend.x + 
        textureSample(tex, _sampler, uvs[1]) * blend.y + 
        textureSample(tex, _sampler, uvs[2]) * blend.z;
}

fn _normal_sample(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(tex_normal, _sampler, uv).rgb * 2.0 - 1.0;
}

@fragment
fn fragment(
    in: MyVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    let worldpos  = in.world_position.xyz;
    let worldnorm = in.world_normal;
    let mtls = round(in.mtls / in.bary);
    let bary = in.bary;

    var blend_triplanar = pow(abs(worldnorm), vec3<f32>(triplanar_blend_pow));  // pow: [4-12], 12=sharper
    // blend_triplanar = max(blend_triplanar - vec3<f32>(triplanar_blend_sharpness), vec3<f32>(0.0));  // sharpen the blend [-0.2 smoother, -0.55 sharper]
    blend_triplanar /= blend_triplanar.x + blend_triplanar.y + blend_triplanar.z;  // makesure sum = 1

#ifdef BLEND 

#else
// #ifdef MAX_BARY
//     let vi_bary_max = _vec3_max_idx(bary);
//     let vi_mtl = vi_bary_max;
// #else
    let vDRAM = array<vec4<f32>, 3>(
        triplanar_sample(tex_dram, mtls[0], worldpos, blend_triplanar),
        triplanar_sample(tex_dram, mtls[1], worldpos, blend_triplanar),
        triplanar_sample(tex_dram, mtls[2], worldpos, blend_triplanar),
    );
	let _blend_heightmap = bary; //pow(bary, vec3<f32>(heightmap_blend_pow));  // BlendHeightmap. Pow: littler=mix, greater=distinct, opt 0.3 - 0.6, 0.48 = nature  ;; Err: pow lead to edge glitch
    let vi_height_max = _vec3_max_idx(vec3<f32>(vDRAM[0].x * _blend_heightmap.x, vDRAM[1].x * _blend_heightmap.y, vDRAM[2].x * _blend_heightmap.z));
    let vi_mtl = vi_height_max;

    let dram = select(select(vDRAM[2], vDRAM[1], vi_mtl==1), vDRAM[0], vi_mtl==0);  // vDRAM[vi_mtl]
    let roughness = dram.g;//pow(dram.y, 50.);
    let metallic  = dram.a;
    let occlusion = dram.b;
// #endif
#endif

    let base_color = triplanar_sample(tex_diffuse, mtls[vi_mtl], worldpos, blend_triplanar);
    
    // NORMAL
    // let normal_intensity = vec3<f32>(1., 1., 1.0);
    blend_triplanar = blend_triplanar * normal_intensity;
    let uvs = triplanar_uv(mtls[vi_mtl], worldpos);
    let tnormX = _normal_sample(uvs[0]);
    let tnormY = _normal_sample(uvs[1]);
    let tnormZ = _normal_sample(uvs[2]);
    // GPU Gems 3, Triplanar Normal Mapping Method.
    let world_normal = normalize(
        vec3<f32>(0., tnormX.y, -tnormX.x) * blend_triplanar.x +
        vec3<f32>(tnormY.x, 0., -tnormY.y) * blend_triplanar.y +
        vec3<f32>(tnormZ.xy, 0.)           * blend_triplanar.z +
        in.world_normal
    );

    var vert_out: VertexOutput;
    vert_out.position = in.position;
    vert_out.world_position = in.world_position;
    vert_out.world_normal = world_normal;
    // vert_out.world_normal = normalize(world_normal + in.world_normal);
    vert_out.instance_index = in.instance_index;
    var pbr_in = pbr_fragment::pbr_input_from_vertex_output(vert_out, is_front, false);

    pbr_in.material.base_color = base_color; 
    pbr_in.material.perceptual_roughness = roughness;
    pbr_in.material.reflectance = 1.0 - roughness;
    // pbr_in.material.ior = 0.99;
    pbr_in.material.metallic = select(0.0, 0.9, (mtls[vi_mtl]) == 9. || (mtls[vi_mtl]) == 8.);
    // pbr_in.occlusion = vec3<f32>(occlusion);
    
    var color = pbr_functions::apply_pbr_lighting(pbr_in);
    color = pbr_functions::main_pass_post_lighting_processing(pbr_in, color);
    
    // var color = base_color;
    // color = vec4<f32>(vec3<f32>(select(0.0, 1.0, round(mtls[vi_mtl]) == 10.)), 1.0); 
    // color = vec4<f32>(bary, 1.0); 
    // color = vec4<f32>(world_normal, 1.0); 
    return color;
}