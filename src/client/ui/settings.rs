use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Color32, Layout, Ui, Widget},
    EguiContexts, EguiSettings,
};

use crate::{
    client::character_controller::CharacterController,
    client::game_client::{ClientInfo, WorldInfo},
    voxel::ClientChunkSystem,
};

use super::{new_egui_window, sfx_play, ui_lr_panel};

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum SettingsPanel {
    #[default]
    General,
    CurrentWorld,
    Graphics,
    Audio,
    Controls,
    Language,
    Mods,
    Assets,
    // Credits,
}

pub fn ui_settings(
    mut ctx: EguiContexts,
    mut settings_panel: Local<SettingsPanel>,

    mut cli: ResMut<ClientInfo>,
    mut worldinfo: Option<ResMut<WorldInfo>>,
    mut egui_settings: ResMut<EguiSettings>,
    mut query_char: Query<&mut CharacterController>,
    mut chunk_sys: Option<ResMut<ClientChunkSystem>>,
    // mut global_volume: ResMut<GlobalVolume>,

    // mut cmds: Commands,
    // asset_server: Res<AssetServer>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let is_world_loaded = worldinfo.is_some();
    new_egui_window("Settings").show(ctx.ctx_mut(), |ui| {
        let curr_settings_panel = *settings_panel;

        ui_lr_panel(
            ui,
            true,
            |ui| {
                sfx_play(ui.selectable_value(&mut *settings_panel, SettingsPanel::General, "General"));
                if is_world_loaded {
                    sfx_play(ui.selectable_value(&mut *settings_panel, SettingsPanel::CurrentWorld, "Current World"));
                }
                ui.separator();
                sfx_play(ui.selectable_value(&mut *settings_panel, SettingsPanel::Graphics, "Graphics"));
                sfx_play(ui.selectable_value(&mut *settings_panel, SettingsPanel::Audio, "Audio"));
                sfx_play(ui.selectable_value(&mut *settings_panel, SettingsPanel::Controls, "Controls"));
                sfx_play(ui.selectable_value(&mut *settings_panel, SettingsPanel::Language, "Languages"));
                ui.separator();
                sfx_play(ui.selectable_value(&mut *settings_panel, SettingsPanel::Mods, "Mods"));
                sfx_play(ui.selectable_value(&mut *settings_panel, SettingsPanel::Assets, "Assets"));
            },
            |ui| {
                ui.style_mut().spacing.item_spacing.y = 12.;

                ui.add_space(16.);

                fn ui_setting_line(ui: &mut Ui, text: impl Into<egui::RichText>, widget: impl Widget) {
                    ui.horizontal(|ui| {
                        ui.add_space(20.);
                        ui.colored_label(Color32::WHITE, text);
                        let end_width = 150.;
                        let end_margin = 8.;
                        let line_margin = 10.;

                        let p = ui.cursor().left_center() + egui::Vec2::new(line_margin, 0.);
                        let p2 = egui::pos2(p.x + ui.available_width() - end_width - line_margin * 2. - end_margin, p.y);
                        ui.painter().line_segment([p, p2], ui.visuals().widgets.noninteractive.bg_stroke);

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(end_margin);
                            ui.add_sized([end_width, 22.], widget);
                        });
                    });
                }

                match curr_settings_panel {
                    SettingsPanel::General => {
                        ui.label("Profile: ");

                        ui_setting_line(ui, "Username", egui::TextEdit::singleline(&mut cli.cfg.username));

                        // ui.group(|ui| {
                        //     ui.horizontal(|ui| {
                        //         ui.vertical(|ui| {
                        //             ui.colored_label(Color32::WHITE, cli.cfg.username.clone());
                        //             ui.small("ref.dreamtowards@gmail.com");
                        //         });

                        //         ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
                        //             ui.button("Log out").clicked();
                        //             if ui.button("Account Info").clicked() {
                        //                 ui.ctx().open_url(egui::OpenUrl::new_tab("https://ethertia.com/profile/uuid"));
                        //             }
                        //         });
                        //     });

                        //     // if ui.button("Switch Account").clicked() {
                        //     //     ui.ctx().open_url(egui::OpenUrl::new_tab("https://auth.ethertia.com/login?client"));
                        //     // }
                        // });

                        // ui.label("General:");

                        ui.label("Voxel:");

                        // ui_setting_line(
                        //     ui,
                        //     "Chunks Meshing Max Concurrency",
                        //     egui::Slider::new(&mut chunk_sys.max_concurrent_meshing, 0..=50),
                        // );

                        if let Some(chunk_sys) = &mut chunk_sys {
                            ui_setting_line(ui, "Chunk Load Distance X", egui::Slider::new(&mut chunk_sys.chunks_load_distance.x, -1..=25));
                            ui_setting_line(ui, "Chunk Load Distance Y", egui::Slider::new(&mut chunk_sys.chunks_load_distance.y, -1..=25));    
                        }
                        
                        ui_setting_line(ui, "Brush Size", egui::Slider::new(&mut cli.brush_size, 0.0..=20.0));

                        ui_setting_line(ui, "Brush Indensity", egui::Slider::new(&mut cli.brush_strength, 0.0..=1.0));

                        ui_setting_line(ui, "Brush Shape", egui::Slider::new(&mut cli.brush_shape, 0..=5));

                        ui_setting_line(ui, "Brush Tex", egui::Slider::new(&mut cli.brush_tex, 0..=25));
                    }
                    SettingsPanel::CurrentWorld => {
                        if let Some(worldinfo) = &mut worldinfo {
                            ui_setting_line(ui, "Day Time", egui::Slider::new(&mut worldinfo.daytime, 0.0..=1.0));

                            ui_setting_line(ui, "Day Time Length", egui::Slider::new(&mut worldinfo.daytime_length, 0.0..=60.0 * 24.0));
                        }
                    }
                    SettingsPanel::Graphics => {
                        ui.label("Video:");

                        ui_setting_line(ui, "FOV", egui::Slider::new(&mut cli.cfg.fov, 10.0..=170.0));

                        ui_setting_line(ui, "VSync", egui::Checkbox::new(&mut cli.vsync, ""));

                        ui_setting_line(ui, "Skylight Shadow", egui::Checkbox::new(&mut cli.skylight_shadow, ""));

                        ui_setting_line(ui, "Skylight Illuminance", egui::Slider::new(&mut cli.skylight_illuminance, 0.1..=200.0));

                        ui.label("UI");

                        ui_setting_line(ui, "UI Scale", egui::Slider::new(&mut egui_settings.scale_factor, 0.5..=2.5));

                        ui_setting_line(ui, "HUD Padding", egui::Slider::new(&mut cli.cfg.hud_padding, 0.0..=48.0));
                    }
                    SettingsPanel::Audio => {

                        // ui_setting_line(ui, "Global Volume", egui::Slider::new(&mut global_volume.volume as &mut f32, 0.0..=1.0));
                    }
                    SettingsPanel::Controls => {
                        if let Ok(mut ctl) = query_char.get_single_mut() {
                            ui_setting_line(ui, "Unfly on Grounded", egui::Checkbox::new(&mut ctl.unfly_on_ground, ""));
                        }
                    }
                    SettingsPanel::Language => {}
                    SettingsPanel::Mods => {}
                    _ => (),
                }
            },
        );
    });
}
