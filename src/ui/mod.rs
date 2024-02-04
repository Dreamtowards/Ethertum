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
    game::{condition, ClientInfo, EthertiaClient, WorldInfo},
    voxel::{ChunkSystem, HitResult},
};

use self::hud::ChatHistory;


mod serverlist;
mod main_menu;
mod debug;
pub mod hud;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        // Setup Egui Style
        app.add_systems(Startup, setup_egui_style);

        app.add_systems(Update, debug::ui_menu_panel); // Debug MenuBar. before CentralPanel
        app.add_systems(Update, main_menu::ui_pause_menu.run_if(in_state(CurrentUI::PauseMenu)).before(debug::ui_menu_panel));

        app.add_state::<CurrentUI>();
        app.add_systems(Update, main_menu::ui_main_menu.run_if(in_state(CurrentUI::MainMenu)));
        app.add_systems(Update, ui_settings.run_if(in_state(CurrentUI::WtfSettings)));

        app.add_plugins((
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
            //SystemInformationDiagnosticsPlugin
        ));
        app.add_systems(Update, debug::hud_debug_text.run_if(|cli: Res<ClientInfo>| cli.dbg_text));

        // HUDs
        {
            app.add_systems(Update, hud::hud_hotbar.run_if(condition::in_world()));
            
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



pub fn new_egui_window(title: &str) -> egui::Window {
    egui::Window::new(title)
        .fixed_size([800., 600.])
        .title_bar(false) 
        .anchor(Align2::CENTER_CENTER, [0., 0.])
        .resizable(false)
        .collapsible(false)
}

pub fn color32_of(c: Color) -> Color32 {
    let c = c.as_rgba_u8();
    Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}

pub fn color32_gray_alpha(gray: f32, alpha: f32) -> Color32 {
    let g = (gray * 255.) as u8;
    let a = (alpha * 255.) as u8;
    Color32::from_rgba_premultiplied(g, g, g, a)
}

fn setup_egui_style(mut egui_settings: ResMut<EguiSettings>, mut ctx: EguiContexts) {
    ctx.ctx_mut().style_mut(|style| {
        let mut visuals = &mut style.visuals;
        let round = Rounding::from(2.);

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

        visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, Color32::from_white_alpha(180));
        visuals.widgets.active.bg_stroke = Stroke::new(3.0, Color32::WHITE);

        visuals.widgets.inactive.weak_bg_fill = Color32::from_white_alpha(10);  // button
        visuals.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(20); // button hovered
        visuals.widgets.active.weak_bg_fill = Color32::from_white_alpha(60); // button pressed

        visuals.selection.bg_fill = Color32::from_rgb(27,76,201);
        visuals.selection.stroke = Stroke::new(2.0, color32_gray_alpha(1., 0.78));  // visuals.selection.bg_fill

        visuals.extreme_bg_color = color32_gray_alpha(0.02, 0.66);  // TextEdit, ProgressBar, ScrollBar Bg, Plot Bg

        visuals.window_fill = color32_gray_alpha(0.1, 0.8);
        visuals.window_shadow = egui::epaint::Shadow{ extrusion: 8., color: Color32::from_black_alpha(45) };
        visuals.popup_shadow = visuals.window_shadow;
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





#[derive(Default, PartialEq, Debug)]
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

pub fn ui_settings(
    mut ctx: EguiContexts, 
    mut settings_panel: Local<SettingsPanel>, 
    mut next_ui: ResMut<NextState<CurrentUI>>, 

    mut clientinfo: ResMut<ClientInfo>,
) {
    new_egui_window("Settings").resizable(true).show(ctx.ctx_mut(), |ui| {

        ui.horizontal(|ui| {
            ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                ui.set_width(128.);

                ui.selectable_value(&mut *settings_panel, SettingsPanel::Profile, "General");
                ui.separator();
                ui.selectable_value(&mut *settings_panel, SettingsPanel::Graphics, "Graphics");
                ui.selectable_value(&mut *settings_panel, SettingsPanel::Audio, "Audio");
                ui.selectable_value(&mut *settings_panel, SettingsPanel::Controls, "Controls");
                ui.selectable_value(&mut *settings_panel, SettingsPanel::Language, "Languages");
                ui.separator();
                ui.selectable_value(&mut *settings_panel, SettingsPanel::Mods, "Mods");
                ui.selectable_value(&mut *settings_panel, SettingsPanel::Assets, "Assets");

                // ui.separator();
                // ui.selectable_value(&mut *settings_panel, SettingsPanel::Credits, "Credits");
                // ui.separator();
                
                ui.add_space(60.);
                ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
                    if ui.selectable_label(false, "Cancel").clicked() {
                        next_ui.set(CurrentUI::MainMenu);
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_min_width(580.);
                
                ui.add_space(6.);
                ui.heading(format!("{:?}", *settings_panel));
                ui.add_space(6.);

                match *settings_panel {
                    SettingsPanel::Profile => {

                        ui.label("Profile");
                        ui.group(|ui| {
                            
                            ui.label("ref.dreamtowards@gmail.com (2736310270)");

                            ui.label("Username: ");
                            ui.text_edit_singleline(&mut clientinfo.username);
                            
                            ui.separator();
                            
                            ui.horizontal(|ui| {
                                if ui.button("Account Info").clicked() {
                                    ui.ctx().open_url(egui::OpenUrl::new_tab("https://ethertia.com/profile/uuid"));
                                }
                                if ui.button("Log out").clicked() {
                                }
                            });
                            // if ui.button("Switch Account").clicked() {
                            //     ui.ctx().open_url(egui::OpenUrl::new_tab("https://auth.ethertia.com/login?client"));
                            // }
                        });
                    }
                    SettingsPanel::Graphics => {
                        ui.heading("Graphics");
                    }
                    SettingsPanel::Audio => {
                        ui.heading("Music & Sound");
                    }
                    SettingsPanel::Controls => {
                        ui.heading("Controls");
                    }
                    SettingsPanel::Language => {
                        ui.heading("Language");
                    }
                    SettingsPanel::Mods => {
                        ui.heading("Mods");
                    }
                    _ => (),
                }
            });
        });
    });
}

