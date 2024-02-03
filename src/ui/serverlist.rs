use bevy::prelude::*;
use bevy::ecs::schedule::NextState;
use bevy_egui::{egui::{self, Align2, Color32, Layout}, EguiContexts};
use bevy_renet::renet::RenetClient;

use crate::game::ClientInfo;

use super::CurrentUI;

use super::new_egui_window;


pub fn ui_connecting_server(
    mut ctx: EguiContexts,
    mut next_ui: ResMut<NextState<CurrentUI>>,
    mut net_client: ResMut<RenetClient>,
) {
    new_egui_window("Server List").show(ctx.ctx_mut(), |ui| {
        let h = ui.available_height();

        ui.vertical_centered(|ui| {
            ui.add_space(h * 0.2);
            
            if net_client.is_connected() {
                ui.label("Authenticating & logging in...");
            } else {
                ui.label("Connecting server...");
            }
            
            ui.add_space(h * 0.3);
            
            if ui.button("Cancel").clicked() {
                // todo: Interrupt Connection without handle Result.
                next_ui.set(CurrentUI::MainMenu);
                net_client.disconnect();
            }

        });

    });
}

pub fn ui_disconnected_reason(
    mut ctx: EguiContexts,
    mut next_ui: ResMut<NextState<CurrentUI>>,

    clientinfo: Res<ClientInfo>,
    net_client: Res<RenetClient>,
) {
    new_egui_window("Disconnected Reason").show(ctx.ctx_mut(), |ui| {
        let h = ui.available_height();

        ui.vertical_centered(|ui| {
            ui.add_space(h * 0.3);

            ui.label("Disconnected:");
            ui.colored_label(Color32::WHITE, clientinfo.disconnected_reason.as_str());
            if let Some(reason) = net_client.disconnect_reason() {
                ui.label(reason.to_string());
            }
            
            ui.add_space(h * 0.3);
            
            if ui.button("Back to title").clicked() {
                // todo: Interrupt Connection without handle Result.
                next_ui.set(CurrentUI::MainMenu);
            }

        });

    });
}




pub fn ui_serverlist(mut ctx: EguiContexts, mut next_ui: ResMut<NextState<CurrentUI>>,) {
    new_egui_window("Server List").show(ctx.ctx_mut(), |ui| {

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let btnsize = [160., 22.];
                if ui.add_sized(btnsize, egui::Button::new("Add Server")).clicked() {
                }
                if ui.add_sized(btnsize, egui::Button::new("Direct Connect")).clicked() {
                }
                if ui.add_sized(btnsize, egui::Button::new("Refresh")).clicked() {
                }
                if ui.add_sized(btnsize, egui::Button::new("Cancel")).clicked() {
                    next_ui.set(CurrentUI::MainMenu);
                }
            });
            
            ui.vertical(|ui| {
                ui.set_min_width(550.);
                
                for i in 0..8 {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::WHITE, "Server Name").on_hover_text("IP: 192.168.1.10");
                            ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label("21ms 12/64");
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label("A Dedicated Server");
                            ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                                if ui.button("Del").clicked() {

                                }
                                if ui.button("Edit").clicked() {

                                }
                                if ui.button("Join").clicked() {

                                }
                            });
                        });
                    });
                }
            });
        });
    });
}