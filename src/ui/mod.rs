
use bevy::prelude::*;

use bevy_egui::{EguiContexts, egui::{self, Widget, Ui}};

use crate::game::{AppState, WorldInfo};


pub fn ui_main_menu(
    mut ctx: EguiContexts,
    mut worldinfo: ResMut<WorldInfo>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    egui::TopBottomPanel::top("top_panel2").show(ctx.ctx_mut(), |ui| {

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

    egui::Window::new("Main Menu").show(ctx.ctx_mut(), |ui| {

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

        ui.label("Copyright Ethertia. Do not distribute!");

    });
}