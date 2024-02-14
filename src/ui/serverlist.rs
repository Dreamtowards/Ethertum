use std::{borrow::Borrow, fs, sync::{Arc, Mutex}};

use bevy::prelude::*;
use bevy::ecs::schedule::NextState;
use bevy_egui::{egui::{self, Align2, Color32, Layout, Ui, Widget}, EguiContexts};
use bevy_renet::renet::RenetClient;
use bevy::reflect::TypePath;
use crate::game::{ClientInfo, EthertiaClient, ServerListItem};

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
            ui.add_space(h * 0.2);

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

// pub fn ui_panel_info(
// ) {
// }

// fn ui_input_server_line(ui: &mut Ui, widget: impl Widget) {
//     ui.horizontal(|ui| {
//         let end_width = 100.;
//         let end_margin = 1.;

//         ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
//             ui.add_space(end_margin);
//             ui.add_sized([end_width, 10.], widget);
//         });
//     });
// }

pub fn ui_serverlist_add(
    mut ctx: EguiContexts, 
    mut next_ui: ResMut<NextState<CurrentUI>>,
    mut cli: ResMut<ClientInfo>,

    mut _name: Local<String>,
    mut _addr: Local<String>,
) {
    new_egui_window("ServerList ItemEdit").show(ctx.ctx_mut(), |ui| {

        ui.vertical_centered(|ui| {

            ui.text_edit_singleline(&mut *_name);
            
            ui.text_edit_singleline(&mut *_addr);

            ui.set_enabled(!_name.is_empty() && !_addr.is_empty());
            let save = ui.button("Save").clicked();
            if save {
                cli.cfg.serverlist.push(ServerListItem { name: _name.clone(), addr: _addr.clone() });
            }
            ui.set_enabled(true);

            if save || ui.button("Cancel").clicked() {
                
                _name.clear();
                _addr.clear();
                next_ui.set(CurrentUI::WtfServerList);
            }
        });
    });
}

pub fn ui_serverlist(
    mut ctx: EguiContexts, 
    mut next_ui: ResMut<NextState<CurrentUI>>,
    mut cli: EthertiaClient,
    mut edit_i: Local<Option<usize>>,
) { 
    new_egui_window("Server List").resizable(true).show(ctx.ctx_mut(), |ui| {
        let serverlist = &mut cli.data().cfg.serverlist;

        let mut do_new_server = false;

        let mut join_addr = None;
        let mut del_i = None;
        

        ui_lr_panel(ui, false, |ui| {
            if ui.selectable_label(false, "Add Server").clicked() {
                // next_ui_1 =  CurrentUI::ServerListItemAdd;
                do_new_server = true;
            }
            if ui.selectable_label(false, "Direct Connect").clicked() {
                
            }
            if ui.selectable_label(false, "Refresh").clicked() {
                
            }
        }, &mut next_ui, |ui| {
            
            for (idx, server_item) in serverlist.iter_mut().enumerate() {
                let editing = edit_i.is_some_and(|i| i == idx);
                
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // ui_input_server_line(ui, egui::TextEdit::singleline(&mut server_info.name).hint_text("name"));
                        // ui_input_server_line(ui, egui::TextEdit::singleline(&mut server_info.address).hint_text("address"));
                        if editing {
                            ui.text_edit_singleline(&mut server_item.name);
                        } else {
                            ui.colored_label(Color32::WHITE, server_item.name.clone()).on_hover_text(server_item.addr.clone());
                            
                            ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label("21ms 12/64");
                            });
                        }
                    });
                    ui.horizontal(|ui| {
                        if editing {
                            ui.text_edit_singleline(&mut server_item.addr);
                        } else {
                            ui.label("A Dedicated Server");
                        }

                        ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                            if editing {
                                if ui.button("Done").clicked() {
                                    *edit_i = None;
                                }
                            } else {
                                if ui.button("🗑").on_hover_text("Delete").clicked() {
                                    del_i = Some(idx);
                                }
                                if ui.button("🔧").on_hover_text("Edit").clicked() {
                                    *edit_i = Some(idx);
                                }
                                if ui.button("⟲").on_hover_text("Refresh Status").clicked() {
                                    
                                }
                                if ui.button("Join").clicked() {
                                    join_addr = Some(server_item.addr.clone());
                                }
                            }
                        });
                    });
                });
            }
            
        });

        if do_new_server {
            
            serverlist.push(ServerListItem { name: "Server Name".into(), addr: "0.0.0.0:4000".into() });
        }
        if let Some(del_i) = del_i {
            serverlist.remove(del_i);
        }

        if let Some(join_addr) = join_addr {
            // 连接服务器 这两个操作会不会有点松散
            next_ui.set(CurrentUI::ConnectingServer);
            cli.connect_server(join_addr);
        }
        // if next_ui_1 != CurrentUI::None {
        //     next_ui.set(next_ui_1);
        // }
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