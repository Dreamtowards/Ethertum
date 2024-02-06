use bevy::prelude::*;
use bevy::ecs::schedule::NextState;
use bevy_egui::{egui::{self, Align2, Color32, Layout, Ui}, EguiContexts};
use bevy_renet::renet::RenetClient;

use crate::game::ClientInfo;

use super::{ui_lr_panel, CurrentUI};

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
            
            // ui.horizontal(|ui| {

            // });
            if net_client.is_connected() {
                ui.label("Authenticating & logging in...");
            } else {
                ui.label("Connecting server...");
            }
            ui.add_space(38.);
            ui.spinner();
            
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
    mut net_client: ResMut<RenetClient>,
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
                net_client.disconnect();
                next_ui.set(CurrentUI::MainMenu);
            }

        });

    });
}



pub fn ui_serverlist(mut ctx: EguiContexts, mut next_ui: ResMut<NextState<CurrentUI>>,) {
    new_egui_window("Server List").resizable(true).show(ctx.ctx_mut(), |ui| {

        ui_lr_panel(ui, false, |ui| {
            if ui.selectable_label(false, "Add Server").clicked() {
                            
            }
            if ui.selectable_label(false, "Direct Connect").clicked() {
                
            }
            if ui.selectable_label(false, "Refresh").clicked() {
                
            }
        }, &mut next_ui, |ui| {
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
}


pub fn ui_localsaves(
    mut ctx: EguiContexts, 
    mut next_ui: ResMut<NextState<CurrentUI>>,
) {
    new_egui_window("Local Saves").resizable(true).show(ctx.ctx_mut(), |ui| {

        ui_lr_panel(ui, false, |ui| {
            if ui.selectable_label(false, "New World").clicked() {
                                    
            }
            if ui.selectable_label(false, "Refresh").clicked() {
                
            }
        }, &mut next_ui, |ui| {
            for i in 0..8 {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::WHITE, "World Name").on_hover_text(
"Path: /Saves/Saa
Size: 10.3 MiB
Time Modified: 2024.02.01 11:20 AM
Time Created: 2024.02.01 11:20 AM
Inhabited: 10.3 hours");
                        ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.label("3 days ago · 13 MB");
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Survival · Cheats");
                        ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                            if ui.button("Del").clicked() {

                            }
                            if ui.button("Edit").clicked() {

                            }
                            if ui.button("Play").clicked() {

                            }
                        });
                    });
                });
            }
        });
    });
}