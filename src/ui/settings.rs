
use bevy::{
    app::AppExit, diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin}, math::vec2, prelude::*, transform::commands
};
use bevy_egui::{
    egui::{
        self, pos2, style::HandleShape, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, LayerId, Layout, Rangef, Rect,
        Rounding, Stroke, Ui, Widget,
    },
    EguiContexts, EguiPlugin, EguiSettings,
};

use crate::{
    character_controller::CharacterController, game::{condition, ClientInfo, EthertiaClient, WorldInfo}, voxel::{ClientChunkSystem, HitResult}
};

use super::{new_egui_window, ui_lr_panel, CurrentUI};


#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum SettingsPanel {
    #[default]
    General,
    Graphics,
    Audio,
    Controls,
    Language,
    Mods,
    Assets,
    Credits,
}

pub fn ui_settings(
    mut ctx: EguiContexts, 
    mut settings_panel: Local<SettingsPanel>, 
    mut next_ui: ResMut<NextState<CurrentUI>>, 

    mut cli: ResMut<ClientInfo>,
    mut worldinfo: Option<ResMut<WorldInfo>>,

    mut query_cam: Query<&mut CharacterController>,
    mut chunk_sys: ResMut<ClientChunkSystem>,
) {
    new_egui_window("Settings").resizable(true).show(ctx.ctx_mut(), |ui| {

        let curr_settings_panel = settings_panel.clone(); 

        ui_lr_panel(ui, true, |ui| {
            ui.selectable_value(&mut *settings_panel, SettingsPanel::General, "General");
            ui.separator();
            ui.selectable_value(&mut *settings_panel, SettingsPanel::Graphics, "Graphics");
            ui.selectable_value(&mut *settings_panel, SettingsPanel::Audio, "Audio");
            ui.selectable_value(&mut *settings_panel, SettingsPanel::Controls, "Controls");
            ui.selectable_value(&mut *settings_panel, SettingsPanel::Language, "Languages");
            ui.separator();
            ui.selectable_value(&mut *settings_panel, SettingsPanel::Mods, "Mods");
            ui.selectable_value(&mut *settings_panel, SettingsPanel::Assets, "Assets");
        }, &mut next_ui, |ui| {

            ui.style_mut().spacing.item_spacing.y = 12.;

            ui.add_space(16.);
            // ui.heading(format!("{:?}", curr_settings_panel));
            // ui.add_space(6.);

            match curr_settings_panel {
                SettingsPanel::General => {

                    ui.label("Profile: ");

                    fn ui_setting_line(ui: &mut Ui, text: impl Into<egui::RichText>, widget: impl Widget) {
                        ui.horizontal(|ui| {
                            ui.add_space(20.);
                            ui.colored_label(Color32::WHITE, text);
                            let end_width = 150.;
                            let end_margin = 8.;
                            let line_margin = 10.;

                            let p = ui.cursor().left_center() + egui::Vec2::new(line_margin, 0.);
                            let p2 = egui::pos2(p.x + ui.available_width() - end_width - line_margin*2. - end_margin, p.y);
                            ui.painter().line_segment([p, p2], ui.visuals().widgets.noninteractive.bg_stroke);
    
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add_space(end_margin);
                                ui.add_sized([end_width, 22.], widget);
                            });
                        });
                    }

                    ui_setting_line(ui, "Username", egui::TextEdit::singleline(&mut cli.cfg.username));


                    ui.label("General:");

                    ui_setting_line(ui, "FOV", egui::Slider::new(&mut cli.cfg.fov, 10.0..=170.0));
                    
                    if let Ok(mut ctl) = query_cam.get_single_mut() {
                        ui_setting_line(ui, "Unfly on Grounded", egui::Checkbox::new(&mut ctl.unfly_on_ground, "Auto Unfly"));
                    }
                   
                    ui.label("Voxel:");

                    ui_setting_line(ui, "Chunks Meshing Max Concurrency", egui::Slider::new(&mut chunk_sys.max_concurrent_meshing, 0..=50));

                    ui_setting_line(ui, "Brush Size", egui::Slider::new(&mut cli.brush_size, 0.0..=20.0));
            
                    ui_setting_line(ui, "Brush Indensity", egui::Slider::new(&mut cli.brush_strength, 0.0..=1.0));
                    
                    ui_setting_line(ui, "Brush Shape", egui::Slider::new(&mut cli.brush_shape, 0..=5));
                    
                    ui_setting_line(ui, "Brush Tex", egui::Slider::new(&mut cli.brush_tex, 0..=25));
            
                    ui.label("UI");

                    ui_setting_line(ui, "HUD Padding", egui::Slider::new(&mut cli.cfg.hud_padding, 0.0..=48.0));

                    ui.label("World");

                    if let Some(worldinfo) = &mut worldinfo {

                        ui_setting_line(ui, "Day Time", egui::Slider::new(&mut worldinfo.daytime, 0.0..=1.0));
                        
                        ui_setting_line(ui, "Day Time Length", egui::Slider::new(&mut worldinfo.daytime_length, 0.0..=60.0*24.0));
                    }

                    //     ui.add(egui::DragValue::new(&mut chunk_sys.view_distance.x).speed(1.));
                    //     ui.add(egui::DragValue::new(&mut chunk_sys.view_distance.y).speed(1.));

                    // ui_setting_line(ui, "Chunks Loading Max Concurrency", egui::Slider::new(&mut chunk_sys.max_concurrent_loading, 0..=50));
                    
                    // ui.indent("ProfileIndent", |ui| {

                    //     ui.group(|ui| {
                        
                    //         // ui.label("ref.dreamtowards@gmail.com (2736310270)");

                    //         ui.label("Username: ");
                    //         ui.text_edit_singleline(&mut clientinfo.username);
                            
                    //         ui.separator();
                            
                    //         ui.horizontal(|ui| {
                    //             if ui.button("Account Info").clicked() {
                    //                 ui.ctx().open_url(egui::OpenUrl::new_tab("https://ethertia.com/profile/uuid"));
                    //             }
                    //             if ui.button("Log out").clicked() {
                    //             }
                    //         });
                    //         // if ui.button("Switch Account").clicked() {
                    //         //     ui.ctx().open_url(egui::OpenUrl::new_tab("https://auth.ethertia.com/login?client"));
                    //         // }
                    //     });
                    // });
                }
                SettingsPanel::Graphics => {
                }
                SettingsPanel::Audio => {
                }
                SettingsPanel::Controls => {
                }
                SettingsPanel::Language => {
                }
                SettingsPanel::Mods => {
                }
                _ => (),
            }
        });

    });
}

