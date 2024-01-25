use bevy::{
    diagnostic::{
        DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    },
    prelude::*,
    render::{renderer::RenderAdapterInfo, view::VisibleEntities},
};

use bevy_egui::{
    egui::{style::HandleShape, FontData, FontDefinitions, FontFamily, Rounding},
    EguiContexts, EguiPlugin, EguiSettings,
};

use crate::voxel::HitResult;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        //app.add_plugins(EguiPlugin);
        //app.add_systems(Update, ui_example_system);

        // Editor
        use bevy_editor_pls::prelude::*;
        app.add_plugins(
            EditorPlugin::default(), // .in_new_window(Window {
                                     //     title: "Editor".into(),
                                     //     ..default()
                                     // })
        );

        app.add_plugins((
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            //SystemInformationDiagnosticsPlugin
        ));

        // Setup Controls
        app.insert_resource(res_editor_controls());
        app.add_systems(Startup, setup_editor_camera_controls);

        // DebugText
        app.add_systems(Startup, setup_debug_text);
        app.add_systems(Update, update_debug_text);

        // Setup Egui Style
        app.add_systems(Startup, setup_egui_style);

        // app.add_systems(Update, ui_example_system);
    }
}

fn res_editor_controls() -> bevy_editor_pls::controls::EditorControls {
    use bevy_editor_pls::controls::*;
    let mut editor_controls = EditorControls::default_bindings();
    editor_controls.unbind(Action::PlayPauseEditor);

    editor_controls.insert(
        Action::PlayPauseEditor,
        Binding {
            input: UserInput::Single(Button::Keyboard(KeyCode::Escape)),
            conditions: vec![BindingCondition::ListeningForText(false)],
        },
    );

    editor_controls
}

fn setup_editor_camera_controls(
    mut query: Query<
        &mut bevy_editor_pls::default_windows::cameras::camera_3d_free::FlycamControls,
    >,
) {
    let mut controls = query.single_mut();
    controls.key_up = KeyCode::E;
    controls.key_down = KeyCode::Q;
}

fn setup_egui_style(
    mut egui_settings: ResMut<EguiSettings>, 
    mut _ctx: EguiContexts
) {
    let mut ctx = _ctx.ctx_mut();
    ctx.style_mut(|style| {
        let mut visuals = &mut style.visuals;
        let round = Rounding::from(2.5);

        visuals.window_rounding = round;
        visuals.widgets.noninteractive.rounding = round;
        visuals.widgets.inactive.rounding = round;
        visuals.widgets.hovered.rounding = round;
        visuals.widgets.active.rounding = round;
        visuals.widgets.open.rounding = round;

        visuals.collapsing_header_frame = true;
        visuals.handle_shape = HandleShape::Rect { aspect_ratio: 0.5 };
        visuals.slider_trailing_fill = true;
    });

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "my_font".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/menlo.ttf")),
    );

    // Put my font first (highest priority):
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .push("my_font".to_owned());

    ctx.set_fonts(fonts);

    //egui_settings.scale_factor = 3.;
}

// fn ui_example_system(mut ctx: EguiContexts) {
//     egui::Window::new("Hello").show(ctx.ctx_mut(), |ui| {
//         ui.label("world");

//         if ui.button("text").clicked() {}
//     });
// }

//////////////////// DEBUG TEXT ////////////////////

#[derive(Component)]
struct DebugTextTag;

fn setup_debug_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/menlo.ttf");

    commands.spawn((
        TextBundle::from_section(
            "This is\ntext with\nline breaks\nin the top left",
            TextStyle {
                font: font.clone(),
                font_size: 14.,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(42.0),
            left: Val::Px(0.0),
            ..default()
        }),
        DebugTextTag,
    ));
}

trait TimeIntervalDetect {
    fn intervals_passed(&self, interval: f32) -> usize;

    fn just_passed(&self, interval: f32) -> bool {
        self.intervals_passed(interval) != 0
    }

    fn intervals(t: f32, dt: f32, u: f32) -> usize {
        ((t / u).floor() - ((t-dt) / u).floor()) as usize
    }
}
impl TimeIntervalDetect for Time {
    fn intervals_passed(&self, u: f32) -> usize {
        Self::intervals(self.elapsed_seconds(), self.delta_seconds(), u)
    }
}

fn update_debug_text(
    // world: &World,
    // cmds: Commands,

    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut query_text: Query<&mut Text, With<DebugTextTag>>,

    query_cam: Query<(&Transform, &VisibleEntities), With<crate::character_controller::CharacterControllerCamera>>,
    mut last_cam_pos: Local<Vec3>,

    mut sys: Local<sysinfo::System>,
    render_adapter_info: Res<RenderAdapterInfo>,

    chunk_sys: Res<crate::voxel::ChunkSystem>,
    worldinfo: Res<crate::game::WorldInfo>,

    hit_result: Res<HitResult>,
) {
    // static mut sys: sysinfo::System = sysinfo::System::new();
    // static mut LAST_UPDATE: f32 = 0.;
    let dt = 0.5;//time.elapsed_seconds() - unsafe { LAST_UPDATE };
    // if dt > 0.2 {
    //     unsafe { LAST_UPDATE = time.elapsed_seconds() };
    // } else {
    //     return;
    // }
    if time.just_passed(2.0) {
        sys.refresh_cpu();
        sys.refresh_memory();
    }
    if !time.just_passed(dt) {
        return;
    }

    let mut text = query_text.single_mut();

    let mut frame_time = time.delta_seconds_f64();
    if let Some(frame_time_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        if let Some(frame_time_smoothed) = frame_time_diagnostic.smoothed() {
            frame_time = frame_time_smoothed;
        }
    }

    let mut fps = frame_time / 1.0;
    if let Some(fps_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps_smoothed) = fps_diagnostic.smoothed() {
            fps = fps_smoothed;
        }
    }

    // "HOMEPATH", "\\Users\\Dreamtowards",
    // "LANG", "en_US.UTF-8",
    // "USERNAME", "Dreamtowards",

    let num_concurrency = std::thread::available_parallelism().unwrap().get();

    use sysinfo::{CpuExt, SystemExt};

    let cpu_arch = std::env::consts::ARCH;
    let dist_id = std::env::consts::OS;
    let os_ver = sys.long_os_version().unwrap();
    let os_ver_sm = sys.os_version().unwrap();

    let cpu_cores = sys.physical_core_count().unwrap();
    let cpu_name = sys.global_cpu_info().brand().trim().to_string();
    let cpu_usage = sys.global_cpu_info().cpu_usage();

    let mem_used = sys.used_memory() as f64 * BYTES_TO_GIB;
    let mem_total = sys.total_memory() as f64 * BYTES_TO_GIB;

    const BYTES_TO_MIB: f64 = 1.0 / 1024.0 / 1024.0;
    const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

    let mut mem_usage_phys = 0.;
    let mut mem_usage_virtual = 0.;

    if let Some(usage) = memory_stats::memory_stats() {
        // println!("Current physical memory usage: {}", byte_unit::Byte::from_bytes(usage.physical_mem as u128).get_appropriate_unit(false).to_string());
        // println!("Current virtual memory usage: {}", byte_unit::Byte::from_bytes(usage.virtual_mem as u128).get_appropriate_unit(false).to_string());

        mem_usage_phys = usage.physical_mem as f64 * BYTES_TO_MIB;
        mem_usage_virtual = usage.virtual_mem as f64 * BYTES_TO_MIB;
    }

    let gpu_name = &render_adapter_info.0.name;
    let gpu_backend = &render_adapter_info.0.backend.to_str();
    let gpu_driver_name = &render_adapter_info.0.driver;
    let gpu_driver_info = &render_adapter_info.0.driver_info;

    let (cam_trans, cam_visible_entities) = query_cam.single();
    let cam_pos = cam_trans.translation;
    let cam_pos_diff = cam_pos - *last_cam_pos;
    let cam_pos_spd = cam_pos_diff.length() / dt;
    let cam_pos_kph = cam_pos_spd * 3.6;
    let cam_pos_x = cam_pos.x;
    let cam_pos_y = cam_pos.y;
    let cam_pos_z = cam_pos.z;

    let cam_visible_entities_num = cam_visible_entities.entities.len();
    let num_all_entities = 0;//world.entities().len();

    // let curr_path = std::env::current_exe().unwrap().display().to_string();
    let os_lang = std::env::var("LANG").unwrap_or("?lang".into()); // "en_US.UTF-8"
                                                                   //let user_name = std::env::var("USERNAME").unwrap();  // "Dreamtowards"

    let daytime = worldinfo.daytime;
    let world_inhabited = worldinfo.time_inhabited;
    let world_seed = worldinfo.seed;

    let num_chunks_loaded = chunk_sys.num_chunks();
    let num_chunks_loading = chunk_sys.chunks_loading.len();
    let num_chunks_remesh = chunk_sys.chunks_remesh.len();
    let num_chunks_meshing = chunk_sys.chunks_meshing.len();

    let mut hit_str = "none".into();
    if hit_result.is_hit {
        hit_str = format!("p: {}, n: {}, d: {}, vox: {}", 
                          hit_result.position, hit_result.normal, hit_result.distance, hit_result.is_voxel);
    }

    text.sections[0].value = format!(
"fps: {fps:.1}, dt: {frame_time:.4}ms
cam: ({cam_pos_x:.2}, {cam_pos_y:.2}, {cam_pos_z:.2}). spd: {cam_pos_spd:.2} mps, {cam_pos_kph:.2} kph.
visible entities: {cam_visible_entities_num} / all {num_all_entities}.

OS:  {dist_id}.{cpu_arch}, {num_concurrency} concurrency, {cpu_cores} cores; {os_lang}. {os_ver}, {os_ver_sm}.
CPU: {cpu_name}, usage {cpu_usage:.1}%
GPU: {gpu_name}, {gpu_backend}. {gpu_driver_name} {gpu_driver_info}
RAM: {mem_usage_phys:.2} MB, vir {mem_usage_virtual:.2} MB | {mem_used:.2} / {mem_total:.2} GB

Hit: {hit_str},

World: '', daytime: {daytime}. inhabited: {world_inhabited}, seed: {world_seed}
Entity: N; components: N, T: n
Chunk: {num_chunks_loaded} loaded, {num_chunks_loading} loading, {num_chunks_remesh} remesh, {num_chunks_meshing} meshing, -- saving.
"
    );

    *last_cam_pos = cam_pos;
}
