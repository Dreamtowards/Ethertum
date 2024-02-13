use std::{borrow::Borrow, fs, sync::{Arc, Mutex}};

use bevy::prelude::*;
use bevy::ecs::schedule::NextState;
use bevy_egui::{egui::{self, Align2, Color32, Layout, Ui, Widget}, EguiContexts};
use bevy_renet::renet::RenetClient;
use bevy::reflect::TypePath;
use crate::game::ClientInfo;

use super::{ui_lr_panel, CurrentUI};

use super::new_egui_window;

// 后面可能单独拿出来作为一个plugin, 负责文件读取
#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct ServerInfo {
    name: String,
    address: String
}
#[derive(serde::Deserialize, serde::Serialize, Asset, TypePath, Clone)]
pub struct ServerList{
    list: Vec<ServerInfo>,
    input_name: String,
    input_addr: String
}

#[derive(Resource)]
pub struct ServerListHandle(Handle<ServerList>);

pub fn setup_serverlist(mut commands: Commands, asset_server: Res<AssetServer>) { 
    // load serverlist json
    commands.insert_resource(ServerListHandle(asset_server.load("client.serverlist.json")));
}


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

fn ui_input_server_line(ui: &mut Ui, widget: impl Widget) {
    ui.horizontal(|ui| {
        let end_width = 100.;
        let end_margin = 1.;

        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add_space(end_margin);
            ui.add_sized([end_width, 10.], widget);
        });
    });
}

pub fn ui_serverlist(
    mut ctx: EguiContexts, 
    mut next_ui: ResMut<NextState<CurrentUI>>,
    server_list_handle: Res<ServerListHandle>,
    mut server_list: ResMut<Assets<ServerList>>,
) { 
    if let Some(server_list) = server_list.get_mut(server_list_handle.0.id()) {
        let servers = Arc::new(Mutex::new(server_list));
        new_egui_window("Server List").resizable(true).show(ctx.ctx_mut(), |ui| {
            ui_lr_panel(ui, false, |ui| {
                let mut add_server = servers.lock().unwrap();
                ui_input_server_line(ui, egui::TextEdit::singleline(&mut add_server.input_name).hint_text("name"));
                ui_input_server_line(ui, egui::TextEdit::singleline(&mut add_server.input_addr).hint_text("address"));
                let add_name = add_server.input_name.clone();
                let add_address = add_server.input_addr.clone();
                if ui.selectable_label(false, "Add Server").clicked() {
                    add_server.list.push(ServerInfo {
                        name: add_name, 
                        address: add_address
                   });
                   let _ = std::fs::write("./assets/client.serverlist.json", serde_json::to_string(&add_server.clone()).unwrap());
                }
                if ui.selectable_label(false, "Direct Connect").clicked() {
                    
                }
                if ui.selectable_label(false, "Refresh").clicked() {
                    
                }
            }, &mut next_ui, |ui| {
                let mut del_server = servers.lock().unwrap();
                let mut del_i = None;
                for i in 0..del_server.list.len() {
                    let server_info = &mut del_server.list[i];
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui_input_server_line(ui, egui::TextEdit::singleline(&mut server_info.name).hint_text("name"));
                                ui_input_server_line(ui, egui::TextEdit::singleline(&mut server_info.address).hint_text("address"));
                            // ui.colored_label(Color32::WHITE, server_info.name.clone()).on_hover_text(server_info.address.clone());
                            ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label("21ms 12/64");
                            });
                        });
                        ui.horizontal(|ui| {
                        ui.label("A Dedicated Server");
                        ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                            if ui.button("Del").clicked() {
                                del_i = Some(i);
                            }
                            if ui.button("Join").clicked() {

                            }
                            });
                        });
                    });
                }
                if let Some(del_i) = del_i {
                    del_server.list.remove(del_i);
                let _ = std::fs::write("./assets/client.serverlist.json", serde_json::to_string(&del_server.clone()).unwrap());
                }
            });
        });
    }
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