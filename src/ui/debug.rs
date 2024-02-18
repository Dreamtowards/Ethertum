
use bevy::{
    app::AppExit, diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin}, prelude::*, transform::commands
};
use bevy_egui::{
    egui::{
        self, Align2, Color32, FontId, Frame, Layout, Widget,
    },
    EguiContexts,
};
use bevy_renet::renet::{transport::NetcodeClientTransport, RenetClient};

use crate::{
    character_controller::CharacterControllerCamera, game::{condition, ClientInfo, EthertiaClient, WorldInfo}, ui::color32_of, voxel::{worldgen, Chunk, ChunkSystem, ClientChunkSystem, HitResult}
};


pub fn ui_menu_panel(
    mut ctx: EguiContexts,
    mut worldinfo: Option<ResMut<WorldInfo>>,
    mut chunk_sys: ResMut<ClientChunkSystem>,
    mut clientinfo: ResMut<ClientInfo>,
    query_cam: Query<&Transform, With<CharacterControllerCamera>>,

    net_client: Option<Res<RenetClient>>,
    net_transport: Option<Res<NetcodeClientTransport>>,
) {
    const BLUE: Color = Color::rgb(0.188, 0.478, 0.776);
    const PURPLE: Color = Color::rgb(0.373, 0.157, 0.467);
    const DARK_RED: Color = Color::rgb(0.525, 0.106, 0.176);
    const ORANGE: Color = Color::rgb(0.741, 0.345, 0.133);
    const DARK: Color = Color::rgba(0., 0., 0., 0.800); // 0.176, 0.176, 0.176
    let bg = if worldinfo.is_some() && worldinfo.as_ref().unwrap().is_paused { color32_of(DARK_RED) } else { color32_of(DARK) };
    // if *state_ingame == GameInput::Controlling {to_color32(DARK)} else {to_color32(PURPLE)};

    egui::TopBottomPanel::top("menu_panel")
        .frame(Frame::default().fill(bg))
        .show_separator_line(false)
        // .height_range(Rangef::new(16., 16.))  // 24
        .show(ctx.ctx_mut(), |ui| {
            // ui.painter().text([0., 48.].into(), Align2::LEFT_TOP, "SomeText", FontId::default(), Color32::WHITE);

            egui::menu::bar(ui, |ui| {
                ui.style_mut().spacing.button_padding.x = 6.;
                ui.style_mut().visuals.widgets.noninteractive.fg_stroke.color = Color32::from_white_alpha(130);
                ui.style_mut().visuals.widgets.inactive.fg_stroke.color = Color32::from_white_alpha(210); // MenuButton lighter

                ui.with_layout(Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    ui.add_space(16.);
                    // ui.small("108M\n30K");
                    // ui.small("10M/s\n8K/s");
                    // ui.label("·");
                    // ui.small("9ms\n12ms");
                    // ui.label("127.0.0.1:4000 · 21ms");

                    if let Some(net_transport) = net_transport {
                        let net_client = net_client.unwrap();
                        if net_client.is_connected() {
                            use human_bytes::human_bytes;
                            let ni = net_client.network_info();

                            let bytes_per_sec = ni.bytes_sent_per_second+ni.bytes_received_per_second;
                            ui.menu_button(format!("{}ms {}/s", ni.rtt as u32, human_bytes(bytes_per_sec)), |ui| {
                                let ping = clientinfo.ping;
                                ui.label("server_addr");  // transport.netcode_client.server_addr()
                                ui.add_space(12.);
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}ms", ping.0)).on_hover_text("Latency / RTT");
                                    ui.small(format!("{}ms\n{}ms", ping.2-ping.1, ping.3-ping.2)).on_hover_text("Latency (Client to Server / Server to Client)");
                                    ui.separator();
                                    ui.label(format!("{}/s", human_bytes(bytes_per_sec))).on_hover_text("Bandwidth");
                                    ui.small(format!("{}/s\n{}/s", human_bytes(ni.bytes_sent_per_second), human_bytes(ni.bytes_received_per_second))).on_hover_text("Bandwidth (Upload/Download)");
                                    // ui.separator();
                                    // ui.label("109M").on_hover_text("Transit");
                                    // ui.small("108M\n30K").on_hover_text("Transit (Upload/Download)");
                                });
                                ui.label(format!("loss packet: {}", ni.packet_loss));
                            }); 
                        }
                    }

                    if let Some(worldinfo) = &mut worldinfo {
                        ui.separator();

                        if worldinfo.is_paused {
                            if egui::Button::new("▶").ui(ui).clicked() {
                                worldinfo.is_paused = false;
                            }
                            if egui::Button::new("⏩").ui(ui).clicked() {
                                //⏩
                                worldinfo.paused_steps += 1;
                            }
                        } else {
                            if egui::Button::new("⏸").ui(ui).clicked() {
                                worldinfo.is_paused = true;
                            }
                        }
                    }

                    // put inside a Layout::right_to_left(egui::Align::Center) or the Vertical Align will offset to upper.
                    ui.with_layout(Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                        ui.add_space(12.);
                        ui.menu_button("System", |ui| {
                            ui.menu_button("Connect Server", |ui| {
                                ui.button("Add Server");
                                ui.separator();
                            });
                            ui.menu_button("Open World", |ui| {
                                ui.button("New World");
                                ui.button("Open World..");
                                ui.separator();
                            });
                            ui.button("Edit World..");
                            ui.button("Close World");
                            ui.separator(); // hello world
                            ui.button("Server Start");
                            ui.button("Server Stop");
                            ui.separator();
                            ui.button("Settings");
                            ui.button("Mods");
                            ui.button("Assets");
                            ui.button("Controls");
                            ui.button("About");
                            ui.separator();
                            ui.button("Terminate");
                        });
                        ui.menu_button("World", |ui| {
                            if ui.button("ReMesh All Chunks").clicked() {
                                let ls =  Vec::from_iter(chunk_sys.get_chunks().keys().cloned());
                                for chunkpos in ls {
                                    chunk_sys.mark_chunk_remesh(chunkpos);
                                }
                            }
                            ui.separator();
                            ui.label("Gizmos:");
                            ui.toggle_value(&mut clientinfo.dbg_gizmo_all_loaded_chunks, "Loaded Chunks");
                            ui.toggle_value(&mut clientinfo.dbg_gizmo_curr_chunk, "Curr Chunk");
                            ui.toggle_value(&mut clientinfo.dbg_gizmo_remesh_chunks, "ReMesh Chunks");
                            ui.separator();

                            if ui.button("Gen Tree").clicked() {
                                let p = query_cam.single().translation.as_ivec3();
                                let mut chunk = chunk_sys.get_chunk(Chunk::as_chunkpos(p)).unwrap().write().unwrap();

                                worldgen::gen_tree(&mut chunk, Chunk::as_localpos(p), 0.8);
                            }
                        });
                        ui.menu_button("Render", |ui| {});
                        ui.menu_button("Audio", |ui| {});
                        ui.menu_button("View", |ui| {
                            ui.toggle_value(&mut true, "HUD");
                            ui.toggle_value(&mut false, "Fullscreen");
                            ui.button("Save Screenshot");

                            ui.separator();
                            ui.toggle_value(&mut clientinfo.dbg_text, "Debug Text");
                            ui.toggle_value(&mut clientinfo.dbg_inspector, "Inspector");

                        });
                    });
                });
            });
        });
}





pub fn hud_debug_text(
    mut ctx: EguiContexts,
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,

    mut sys: Local<sysinfo::System>,
    render_adapter_info: Res<bevy::render::renderer::RenderAdapterInfo>,

    worldinfo: Option<Res<WorldInfo>>,
    chunk_sys: Res<ClientChunkSystem>,
    hit_result: Res<HitResult>,
    query_cam: Query<(&Transform, &bevy::render::view::VisibleEntities), With<crate::character_controller::CharacterControllerCamera>>,
    mut last_cam_pos: Local<Vec3>,
) {

    let mut str_sys = String::default();
    #[cfg(feature = "target_native_os")]
    {
        use crate::util::TimeIntervals;
    
        if time.at_interval(2.0) {
            sys.refresh_cpu();
            sys.refresh_memory();
        }

        // "HOMEPATH", "\\Users\\Dreamtowards",
        // "LANG", "en_US.UTF-8",
        // "USERNAME", "Dreamtowards",

        let num_concurrency = std::thread::available_parallelism().unwrap().get();

        use sysinfo::{CpuExt, SystemExt};

        let cpu_arch = std::env::consts::ARCH;
        let dist_id = std::env::consts::OS;
        let os_ver = sys.long_os_version().unwrap_or_default();
        let os_ver_sm = sys.os_version().unwrap_or_default();
        
        // let curr_path = std::env::current_exe().unwrap().display().to_string();
        let os_lang = std::env::var("LANG").unwrap_or("?lang".into()); // "en_US.UTF-8"
        //let user_name = std::env::var("USERNAME").unwrap();  // "Dreamtowards"

        let cpu_cores = sys.physical_core_count().unwrap_or_default();
        let cpu_name = sys.global_cpu_info().brand().trim().to_string();
        let cpu_usage = sys.global_cpu_info().cpu_usage();

        let mem_used = sys.used_memory() as f64 * BYTES_TO_GIB;
        let mem_total = sys.total_memory() as f64 * BYTES_TO_GIB;

        const BYTES_TO_MIB: f64 = 1.0 / 1024.0 / 1024.0;
        const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

        let mut mem_usage_phys = 0.;
        let mut mem_usage_virtual = 0.;

        let gpu_name = &render_adapter_info.0.name;
        let gpu_backend = &render_adapter_info.0.backend.to_str();
        let gpu_driver_name = &render_adapter_info.0.driver;
        let gpu_driver_info = &render_adapter_info.0.driver_info;

        // #[cfg(feature = "target_native_os")]
        if let Some(usage) = memory_stats::memory_stats() {
            // println!("Current physical memory usage: {}", byte_unit::Byte::from_bytes(usage.physical_mem as u128).get_appropriate_unit(false).to_string());
            // println!("Current virtual memory usage: {}", byte_unit::Byte::from_bytes(usage.virtual_mem as u128).get_appropriate_unit(false).to_string());

            mem_usage_phys = usage.physical_mem as f64 * BYTES_TO_MIB;
            mem_usage_virtual = usage.virtual_mem as f64 * BYTES_TO_MIB;
        }

        str_sys = format!(
"\nOS:  {dist_id}.{cpu_arch}, {num_concurrency} concurrency, {cpu_cores} cores; {os_lang}. {os_ver}, {os_ver_sm}.
CPU: {cpu_name}, usage {cpu_usage:.1}%
GPU: {gpu_name}, {gpu_backend}. {gpu_driver_name} {gpu_driver_info}
RAM: {mem_usage_phys:.2} MB, vir {mem_usage_virtual:.2} MB | {mem_used:.2} / {mem_total:.2} GB\n\n");
    }
    
    let mut cam_visible_entities_num = 0;
    let mut str_world = String::default();
    if let Some(worldinfo) = worldinfo {
        let (cam_trans, cam_visible_entities) = query_cam.single();
        let cam_pos = cam_trans.translation;
        let cam_pos_spd = (cam_pos - *last_cam_pos).length() / time.delta_seconds();
        *last_cam_pos = cam_pos;
        cam_visible_entities_num = cam_visible_entities.entities.len();

        let num_chunks_loading = -1;//chunk_sys.chunks_loading.len();
        let num_chunks_remesh = chunk_sys.chunks_remesh.len();
        let num_chunks_meshing = -1;//chunk_sys.chunks_meshing.len();

        let mut hit_str = "none".into();
        if hit_result.is_hit {
            hit_str = format!(
                "p: {}, n: {}, d: {}, vox: {}",
                hit_result.position, hit_result.normal, hit_result.distance, hit_result.is_voxel
            );
        }
        
        let mut cam_cell_str = "none".into();
        if let Some(c) = chunk_sys.get_cell(cam_pos.as_ivec3()) {
            cam_cell_str = format!(
                "tex: {}, shape: {}, isoval: {}",
                c.tex_id, c.shape_id, c.isovalue()
            );
        }

        str_world = format!(
"
Cam: ({:.1}, {:.2}, {:.3}). spd: {:.2} mps, {:.2} kph.
Hit: {hit_str},
CamCell: {cam_cell_str}
World: '{}', daytime: {}. inhabited: {}, seed: {}
Chunk: {} loaded, {num_chunks_loading} loading, {num_chunks_remesh} remesh, {num_chunks_meshing} meshing, -- saving.",
cam_pos.x, cam_pos.y, cam_pos.z, cam_pos_spd, cam_pos_spd * 3.6,
worldinfo.name, worldinfo.daytime, worldinfo.time_inhabited, worldinfo.seed,
chunk_sys.num_chunks());
    }

    let frame_time = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME).map_or(time.delta_seconds_f64(), |d|d.smoothed().unwrap_or_default());

    let fps = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).map_or(frame_time / 1.0, |d|d.smoothed().unwrap_or_default());

    let num_entity = diagnostics.get(EntityCountDiagnosticsPlugin::ENTITY_COUNT).map_or(0., |f|f.smoothed().unwrap_or_default()) as usize;

    let str = format!(
"fps: {fps:.1}, dt: {frame_time:.4}ms
entity vis: {cam_visible_entities_num} / all {num_entity}.
{str_sys}
{str_world}
"
    );

    ctx.ctx_mut().debug_painter().text(
        [0., 48.].into(),
        Align2::LEFT_TOP,
        str,
        FontId::proportional(12.),
        Color32::WHITE,
    );

}
