
use std::{default, sync::Arc};

use bevy::{app::AppExit, prelude::*};
use bevy_egui::{
    egui::{
        self, Align2, Color32, FontId, Frame, Layout, Stroke, Ui, Widget, Rounding, 
        style::HandleShape, FontData, FontDefinitions, FontFamily, 
    },
    EguiContexts,
    EguiSettings,
};

use crate::game::{AppState, GameInput, WorldInfo};

fn to_color32(c: Color) -> Color32 {
    let c = c.as_rgba_u8();
    Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}


pub fn setup_egui_style(
    mut egui_settings: ResMut<EguiSettings>, 
    mut _ctx: EguiContexts
) {
    let mut ctx = _ctx.ctx_mut();
    ctx.style_mut(|style| {
        let mut visuals = &mut style.visuals;
        let round = Rounding::from(0.);

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

pub fn ui_menu_panel(
    mut ctx: EguiContexts,
    mut worldinfo: ResMut<WorldInfo>,
    state_ingame: ResMut<State<GameInput>>,
) {
    const BLUE: Color = Color::rgb(0.188, 0.478, 0.776);
    const PURPLE: Color = Color::rgb(0.373, 0.157, 0.467);
    const DARK_RED: Color = Color::rgb(0.525, 0.106, 0.176);
    const ORANGE: Color = Color::rgb(0.741, 0.345, 0.133);
    const DARK: Color = Color::rgba(0.176, 0.176, 0.176, 0.700);
    let bg = if *state_ingame == GameInput::Controlling {to_color32(DARK)} else {to_color32(PURPLE)};

    egui::TopBottomPanel::top("menu_panel")
        .frame(Frame {
            fill: bg,
            stroke: Stroke::NONE,
            ..default()
        })
        .show(ctx.ctx_mut(), |ui| {

        ui.painter().text([0., 500.].into(), Align2::LEFT_TOP, "SomeText", FontId::default(), Color32::WHITE);

        // ui.style_mut().visuals.panel_fill = to_color32(BLUE);
        // ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
        // ui.visuals_mut().widgets.inactive.bg_fill = to_color32(PURPLE);
        // ui.visuals_mut().widgets.inactive.weak_bg_fill = to_color32(BLUE);

        ui.horizontal(|ui| {
            egui::menu::bar(ui, |ui| {

                ui.add_space(12.);

                ui.menu_button("System", |ui| {
                    ui.menu_button("Connect Server", |ui| {
                        ui.button("+").on_hover_text("Add Server");
                        ui.separator();
                    });
                    ui.menu_button("Open World", |ui| {
                        ui.button("New World");
                        ui.button("Open World..");
                        ui.separator();
                    });
                    ui.button("Edit World..");
                    ui.button("Close World");
                    ui.separator();
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
                ui.menu_button("Render", |ui| {

                });
                ui.menu_button("Audio", |ui| {

                });
                ui.menu_button("View", |ui| {

                    ui.toggle_value(&mut true, "HUD");
                    ui.toggle_value(&mut false, "Fullscreen");
                    ui.button("Save Screenshot");
                    ui.separator();
                    ui.toggle_value(&mut true, "Debug Info");
                });
                
                // ui.label("·");
                ui.add_space(10.);
                ui.separator();
                ui.add_space(10.);


                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {

                        if worldinfo.is_paused {
                            if egui::Button::new("⏸").ui(ui).clicked() {
                                worldinfo.is_paused = false;
                            }
                        } else {
                            if egui::Button::new("▶").ui(ui).clicked() {
                                worldinfo.is_paused = true;
                            }
                            if egui::Button::new("Step").ui(ui).clicked() {  //⏩  
                                worldinfo.paused_steps += 1;
                            }
                        }
                        ui.separator();

                        ui.label("127.0.0.1:4000 · 21ms");
                        ui.small("9ms\n12ms");
                        ui.label("·");
                        ui.small("10M/s\n8K/s");
                        ui.small("108M\n30K");

                        ui.add_space(12.);
                    });
                });

            });
        });

    });
}




pub fn ui_main_menu(
    mut ctx: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit_events: EventWriter<AppExit>,
) {

    egui::CentralPanel::default().show(ctx.ctx_mut(), |ui| {
        let h = ui.available_height();
        let w = ui.available_width();
        

        ui.vertical_centered(|ui| {

            ui.add_space(h * 0.12);
            ui.heading("ethertia");
            ui.add_space(h * 0.2);

            if ui.add_sized([200., 20.], egui::Button::new("Play")).clicked() {
                next_state.set(AppState::InGame);
            }
            if ui.add_sized([200., 20.], egui::Button::new("Settings")).clicked() {
                next_state.set(AppState::WtfSettings);
            }
            if ui.add_sized([200., 20.], egui::Button::new("Terminate")).clicked() {
                app_exit_events.send(AppExit);
            }
            
            // ui.painter().text([w, h].into(), Align2::RIGHT_BOTTOM, 
            // "Copyright M0jang AB. Do not distribute!", FontId::default(), Color32::WHITE);
        });

        // ui.set_max_size([600., 600.].into());
        // ui.cursor().set_top(580.);

        ui.with_layout(Layout::bottom_up(egui::Align::Max), |ui| {
            ui.label("Copyrights nullptr. Do not distribute!");
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

pub fn ui_settings(
    mut ctx: EguiContexts,
    mut settings_panel: Local<SettingsPanel>,
    mut next_state: ResMut<NextState<AppState>>,
) {

    egui::CentralPanel::default().show(ctx.ctx_mut(), |ui| {
        
        ui.add_space(48.);
        ui.heading("Settings");
        ui.add_space(24.);
        
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    if ui.small_button("<").clicked() {
                        next_state.set(AppState::MainMenu);  // or set to InGame if it's openned from InGame state
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
            ui.group(|ui| {
                match *settings_panel {
                    SettingsPanel::Profile => {
                        ui.label("Profile");
                    },
                    SettingsPanel::Graphics => {
                        ui.label("Graphics");
                    },
                    _ => ()
                }
            });
        });

    });
}

pub fn ui_pause_menu(
    mut ctx: EguiContexts,
    mut next_state_game: ResMut<NextState<AppState>>,

    state_ingame: ResMut<State<GameInput>>,
    mut next_state_ingame: ResMut<NextState<GameInput>>,
) {
    if *state_ingame == GameInput::Controlling {
        return;
    }
    // egui::Window::new("Pause Menu").show(ctx.ctx_mut(), |ui| {
    egui::CentralPanel::default()
        .frame(Frame::default().fill(Color32::from_black_alpha(140)))
        .show(ctx.ctx_mut(), |ui| 
    {
        let h = ui.available_height();
        ui.add_space(h * 0.2);

        ui.vertical_centered(|ui| {

            if ui.add_sized([200., 20.], egui::Button::new("Continue")).clicked() {
                next_state_ingame.set(GameInput::Controlling);
            }
            if ui.add_sized([200., 20.], egui::Button::new("Back to Title")).clicked() {
                next_state_game.set(AppState::MainMenu);
            }
        });

    });
}