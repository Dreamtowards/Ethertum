
use std::{default, sync::Arc};

use bevy::{app::AppExit, prelude::*};

use bevy_egui::{EguiContexts, egui::{self, Align2, Color32, FontId, Ui, Widget}};

use crate::game::{AppState, GameInput, WorldInfo};


pub fn ui_menu_panel(
    mut ctx: EguiContexts,
    mut worldinfo: ResMut<WorldInfo>,
) {
    egui::TopBottomPanel::top("menu_panel").show(ctx.ctx_mut(), |ui| {

        ui.horizontal(|ui| {
            egui::menu::bar(ui, |ui| {

                ui.menu_button("System", |ui| {

                });
                ui.menu_button("World", |ui| {

                });
                ui.menu_button("Render", |ui| {

                });
                ui.menu_button("Audio", |ui| {

                });
                ui.menu_button("View", |ui| {

                });
                
                // ui.label("·");
                ui.add_space(20.);
                if worldinfo.is_paused {
                    if egui::Button::new("▶").ui(ui).clicked() {
                        worldinfo.is_paused = false;
                    }
                } else {
                    if egui::Button::new("⏸").ui(ui).clicked() {
                        worldinfo.is_paused = true;
                    }
                    if egui::Button::new("⏩").ui(ui).clicked() {
                        worldinfo.paused_steps += 1;
                    }
                }

                ui.add_space(ui.available_width() - 300.);

                ui.label("127.0.0.1:4000 · 21ms");
                ui.small("9ms\n12ms");
                ui.label("·");
                ui.small("10M/s\n8K/s");
                ui.small("108M\n30K");
            });
        });

    });
}


// #[derive(Resource, Default)]
// pub struct UiDrawList {
//     drawcalls: Vec<Box<dyn FnOnce(&mut Ui)>>
// }


#[derive(Event)]
pub struct UiDrawEvent {
    call: &dyn FnOnce(&mut Ui),
}


pub fn ui_central_draw(
    mut ctx: EguiContexts,
    // mut drawlist: ResMut<UiDrawList>,
    events: EventWriter<UiDrawEvent>
) {
    // drawlist.drawcalls.push(|ui| {

    // });

    egui::CentralPanel::default().show(ctx.ctx_mut(), |ui| {

        for drawcall in &drawlist.drawcalls {
            drawcall(ui);
        }

    });
    drawlist.drawcalls.clear();
}



pub fn ui_main_menu(
    mut ctx: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit_events: EventWriter<AppExit>,
    mut drawlist: ResMut<UiDrawList>,
) {

    drawlist.drawcalls.push(|ui| {
        let h = ui.available_height();
        let w = ui.available_width();
        
        ui.vertical_centered(|ui| {

            ui.heading("ethertia");
            ui.add_space(h * 0.12);
            ui.heading("ethertia");
            ui.add_space(h * 0.2);

            if ui.add_sized([200., 20.], egui::Button::new("Play")).clicked() {
                next_state.set(AppState::InGame);
            }
            if ui.add_sized([200., 20.], egui::Button::new("Settings")).clicked() {
        //         next_state.set(AppState::WtfSettings);
            }
        //     if ui.add_sized([200., 20.], egui::Button::new("Terminate")).clicked() {
        //         app_exit_events.send(AppExit);
        //     }
            
        //     ui.painter().text([w, h].into(), Align2::RIGHT_BOTTOM, 
        //     "Copyright M0jang AB. Do not distribute!", FontId::default(), Color32::WHITE);
        });
    });
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
            
            ui.painter().text([w, h].into(), Align2::RIGHT_BOTTOM, 
            "Copyright M0jang AB. Do not distribute!", FontId::default(), Color32::WHITE);
        });

        // ui.set_max_size([600., 600.].into());
        // ui.cursor().set_top(580.);

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
    mut drawlist: ResMut<UiDrawList>,
) {

    drawlist.drawcalls.push(|ui| {

        ui.label("Settings");
        ui.add_space(48.);
        ui.heading("Settings");
        ui.add_space(24.);
    });
    
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
    egui::Window::new("Pause Menu").show(ctx.ctx_mut(), |ui| {

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