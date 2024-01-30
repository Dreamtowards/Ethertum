
use bevy::prelude::*;

use bevy_egui::{EguiContexts, egui::{self, Widget, Ui}};

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

pub fn ui_main_menu(
    mut ctx: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
) {
    egui::Window::new("Main Menu").title_bar(false).show(ctx.ctx_mut(), |ui| {
        

        ui.vertical_centered(|ui| {

            ui.heading("ethertia");

            if ui.add_sized([200., 20.], egui::Button::new("Play")).clicked() {
                next_state.set(AppState::InGame);
            }
            if ui.add_sized([200., 20.], egui::Button::new("Settings")).clicked() {

            }
            if ui.add_sized([200., 20.], egui::Button::new("Terminate")).clicked() {

            }
        });

        ui.set_max_size([600., 600.].into());
        ui.cursor().set_top(580.);
        ui.label("Copyright Ethertia. Do not distribute!");

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

        ui.label("Copyright Ethertia. Do not distribute!");

    });
}