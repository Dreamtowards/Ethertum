use std::{default, sync::Arc};

use bevy::{
    app::AppExit, diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin}, prelude::*, transform::commands
};
use bevy_egui::{
    egui::{
        self, pos2, style::HandleShape, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, LayerId, Layout, Rangef, Rect,
        Rounding, Stroke, Ui, Widget,
    },
    EguiContexts, EguiPlugin, EguiSettings,
};

use crate::{
    game::{condition, EthertiaClient, WorldInfo},
    voxel::{ChunkSystem, HitResult},
};

use self::hud::ChatHistory;


mod serverlist;
mod main_menu;
pub mod hud;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        // Setup Egui Style
        app.add_systems(Startup, setup_egui_style);

        app.add_systems(Update, ui_menu_panel); // Debug MenuBar. before CentralPanel
        app.add_systems(Update, main_menu::ui_pause_menu.run_if(in_state(CurrentUI::PauseMenu)).before(ui_menu_panel));

        app.add_state::<CurrentUI>();
        app.add_systems(Update, main_menu::ui_main_menu.run_if(in_state(CurrentUI::MainMenu)));
        app.add_systems(Update, ui_settings.run_if(in_state(CurrentUI::WtfSettings)));

        app.add_plugins((
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            //SystemInformationDiagnosticsPlugin
        ));
        app.add_systems(Update, hud_debug_text.run_if(condition::in_world()));

        // HUDs
        {
            app.add_systems(Update, hud_hotbar.run_if(condition::in_world()));
            
            app.insert_resource(ChatHistory::default());
            app.add_systems(Update, hud::hud_chat.run_if(condition::in_world()));
        }
        

        app.add_systems(Update, serverlist::ui_serverlist.run_if(in_state(CurrentUI::WtfServerList)));
        
        app.add_systems(Update, serverlist::ui_connecting_server.run_if(in_state(CurrentUI::ConnectingServer)));
        app.add_systems(Update, serverlist::ui_disconnected_reason.run_if(in_state(CurrentUI::DisconnectedReason)));
    }
}


#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, States)]
pub enum CurrentUI {
    None,
    #[default]
    MainMenu,
    PauseMenu,
    WtfSettings,
    WtfServerList,
    ConnectingServer,
    DisconnectedReason,
    ChatInput,
}




fn to_color32(c: Color) -> Color32 {
    let c = c.as_rgba_u8();
    Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}

fn setup_egui_style(mut egui_settings: ResMut<EguiSettings>, mut ctx: EguiContexts) {
    ctx.ctx_mut().style_mut(|style| {
        let mut visuals = &mut style.visuals;
        let round = Rounding::from(0.);

        visuals.window_rounding = round;
        visuals.widgets.noninteractive.rounding = round;
        visuals.widgets.inactive.rounding = round;
        visuals.widgets.hovered.rounding = round;
        visuals.widgets.active.rounding = round;
        visuals.widgets.open.rounding = round;
        visuals.window_rounding = round;
        visuals.menu_rounding = round;

        visuals.collapsing_header_frame = true;
        visuals.handle_shape = HandleShape::Rect { aspect_ratio: 0.5 };
        visuals.slider_trailing_fill = true;

        visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, Color32::from_white_alpha(200));
        visuals.widgets.active.bg_stroke = Stroke::new(3.0, Color32::WHITE);

        visuals.widgets.inactive.weak_bg_fill = Color32::from_white_alpha(10);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(20); // button hovered
        visuals.widgets.active.weak_bg_fill = Color32::from_white_alpha(60); // button hovered
    });

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "my_font".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/menlo.ttf")),
    );

    // Put my font first (highest priority):
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts.families.get_mut(&FontFamily::Monospace).unwrap().push("my_font".to_owned());

    ctx.ctx_mut().set_fonts(fonts);

    // egui_settings.scale_factor = 1.;
}



fn ui_menu_panel(mut ctx: EguiContexts, mut worldinfo: Option<ResMut<WorldInfo>>) {
    const BLUE: Color = Color::rgb(0.188, 0.478, 0.776);
    const PURPLE: Color = Color::rgb(0.373, 0.157, 0.467);
    const DARK_RED: Color = Color::rgb(0.525, 0.106, 0.176);
    const ORANGE: Color = Color::rgb(0.741, 0.345, 0.133);
    const DARK: Color = Color::rgba(0., 0., 0., 0.800); // 0.176, 0.176, 0.176
    let bg = if worldinfo.is_some() && worldinfo.as_ref().unwrap().is_paused { to_color32(DARK_RED) } else { to_color32(DARK) };
    // if *state_ingame == GameInput::Controlling {to_color32(DARK)} else {to_color32(PURPLE)};

    egui::TopBottomPanel::top("menu_panel")
        .frame(Frame::default().fill(bg))
        .show_separator_line(false)
        // .height_range(Rangef::new(16., 16.))  // 24
        .show(ctx.ctx_mut(), |ui| {
            // ui.painter().text([0., 48.].into(), Align2::LEFT_TOP, "SomeText", FontId::default(), Color32::WHITE);

            egui::menu::bar(ui, |ui| {
                ui.style_mut().spacing.button_padding.x = 6.;
                ui.style_mut().visuals.widgets.noninteractive.fg_stroke.color = Color32::from_white_alpha(180);
                ui.style_mut().visuals.widgets.inactive.fg_stroke.color = Color32::from_white_alpha(210); // MenuButton lighter

                ui.with_layout(Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    ui.add_space(16.);
                    // ui.small("108M\n30K");
                    // ui.small("10M/s\n8K/s");
                    // ui.label("·");
                    // ui.small("9ms\n12ms");
                    // ui.label("127.0.0.1:4000 · 21ms");
                    ui.menu_button("21ms 14K/s", |ui| {
                        ui.label("127.0.0.1:4000");
                        ui.add_space(12.);
                        ui.horizontal(|ui| {
                            ui.label("21ms").on_hover_text("Latency");
                            ui.small("9ms\n12ms").on_hover_text("Latency (Client to Server / Server to Client)");
                            ui.separator();
                            ui.label("1M/s").on_hover_text("Bandwidth");
                            ui.small("1M/s\n8K/s").on_hover_text("Bandwidth (Upload/Download)");
                            ui.separator();
                            ui.label("109M").on_hover_text("Transit");
                            ui.small("108M\n30K").on_hover_text("Transit (Upload/Download)");
                        });
                    });

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
                            ui.button("Resume");
                            ui.button("Step");
                        });
                        ui.menu_button("Render", |ui| {});
                        ui.menu_button("Audio", |ui| {});
                        ui.menu_button("View", |ui| {
                            ui.toggle_value(&mut true, "HUD");
                            ui.toggle_value(&mut false, "Fullscreen");
                            ui.button("Save Screenshot");
                            // ui.separator();
                            // ui.toggle_value(&mut worldinfo.dbg_text, "Debug Info");
                        });
                    });
                });
            });
        });
}


#[derive(Default, PartialEq)]
pub enum SettingsPanel {
    #[default]
    Profile,
    Graphics,
    Audio,
    Controls,
    Language,
    Mods,
    Assets,
    Credits,
}

pub fn ui_settings(mut ctx: EguiContexts, mut settings_panel: Local<SettingsPanel>, mut next_state: ResMut<NextState<CurrentUI>>) {
    egui::CentralPanel::default().show(ctx.ctx_mut(), |ui| {
        ui.add_space(48.);
        ui.heading("Settings");
        ui.add_space(24.);

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    if ui.small_button("<").clicked() {
                        next_state.set(CurrentUI::MainMenu); // or set to InGame if it's openned from InGame state
                    }
                    ui.radio_value(&mut *settings_panel, SettingsPanel::Profile, "Profile");
                    // ui.separator();
                    ui.radio_value(&mut *settings_panel, SettingsPanel::Graphics, "Graphics");
                    ui.radio_value(&mut *settings_panel, SettingsPanel::Audio, "Music & Sounds");
                    ui.radio_value(&mut *settings_panel, SettingsPanel::Controls, "Controls");
                    ui.radio_value(&mut *settings_panel, SettingsPanel::Language, "Languages");
                    // ui.separator();
                    ui.radio_value(&mut *settings_panel, SettingsPanel::Mods, "Mods");
                    ui.radio_value(&mut *settings_panel, SettingsPanel::Assets, "Assets");
                    // ui.separator();
                    ui.radio_value(&mut *settings_panel, SettingsPanel::Credits, "Credits");

                    // ui.set_max_width(180.);
                });
                // ui.set_max_width(180.);
            });
            ui.group(|ui| match *settings_panel {
                SettingsPanel::Profile => {
                    ui.label("Profile");
                }
                SettingsPanel::Graphics => {
                    ui.label("Graphics");
                }
                _ => (),
            });
        });
    });
}

fn hud_hotbar(mut ctx: EguiContexts) {
    egui::Window::new("HUD Hotbar")
        .title_bar(false)
        .anchor(Align2::CENTER_BOTTOM, [0., -16.])
        .show(ctx.ctx_mut(), |ui| {
            let s = 50.;

            ui.horizontal(|ui| {
                for i in 0..9 {
                    ui.add_sized([s, s], egui::Button::new(""));
                }
            });
        });
}

fn hud_debug_text(
    // world: &World,
    // cmds: Commands,
    mut ctx: EguiContexts,

    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,

    query_cam: Query<(&Transform, &bevy::render::view::VisibleEntities), With<crate::character_controller::CharacterControllerCamera>>,
    mut last_cam_pos: Local<Vec3>,

    mut sys: Local<sysinfo::System>,
    render_adapter_info: Res<bevy::render::renderer::RenderAdapterInfo>,

    chunk_sys: Res<ChunkSystem>,
    worldinfo: Res<WorldInfo>,

    hit_result: Res<HitResult>,
) {
    if worldinfo.dbg_text {
        return;
    }

    use crate::util::TimeIntervals;
    if time.at_interval(2.0) {
        sys.refresh_cpu();
        sys.refresh_memory();
    }
    let dt = time.delta_seconds();

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

    #[cfg(not(feature = "web"))]
    {
        if let Some(usage) = memory_stats::memory_stats() {
            // println!("Current physical memory usage: {}", byte_unit::Byte::from_bytes(usage.physical_mem as u128).get_appropriate_unit(false).to_string());
            // println!("Current virtual memory usage: {}", byte_unit::Byte::from_bytes(usage.virtual_mem as u128).get_appropriate_unit(false).to_string());

            mem_usage_phys = usage.physical_mem as f64 * BYTES_TO_MIB;
            mem_usage_virtual = usage.virtual_mem as f64 * BYTES_TO_MIB;
        }
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
    let num_all_entities = 0; //world.entities().len();

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
        hit_str = format!(
            "p: {}, n: {}, d: {}, vox: {}",
            hit_result.position, hit_result.normal, hit_result.distance, hit_result.is_voxel
        );
    }

    let str = format!(
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

    ctx.ctx_mut().debug_painter().text(
        [0., 48.].into(),
        Align2::LEFT_TOP,
        str,
        FontId::proportional(12.),
        Color32::from_gray(230),
    );

    *last_cam_pos = cam_pos;
}
