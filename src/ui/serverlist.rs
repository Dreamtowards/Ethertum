use std::{borrow::Borrow, fs, sync::{Arc, Mutex}};

use bevy::prelude::*;
use bevy_egui::{egui::{self, Align2, Color32, Layout, Ui, Widget}, EguiContexts};
use bevy_renet::renet::RenetClient;
use crate::game_client::{ClientInfo, EthertiaClient, ServerListItem};

use super::{sfx_play, ui_lr_panel, CurrentUI, UiExtra};

use super::new_egui_window;



pub fn ui_connecting_server(
    mut ctx: EguiContexts,
    mut cli: ResMut<ClientInfo>,
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
            
            if ui.btn_normal("Cancel").clicked() {
                // todo: Interrupt Connection without handle Result.
                cli.curr_ui = CurrentUI::MainMenu;
                net_client.disconnect();
            }

        });

    });
}

pub fn ui_disconnected_reason(
    mut ctx: EguiContexts,
    mut cli: ResMut<ClientInfo>,  // readonly. mut only for curr_ui.
) {
    new_egui_window("Disconnected Reason").show(ctx.ctx_mut(), |ui| {
        let h = ui.available_height();

        ui.vertical_centered(|ui| {
            ui.add_space(h * 0.2);

            ui.label("Disconnected:");
            ui.colored_label(Color32::WHITE, cli.disconnected_reason.as_str());

            
            ui.add_space(h * 0.3);
            
            if ui.btn_normal("Back to title").clicked() {
                cli.curr_ui = CurrentUI::MainMenu;
            }
        });

    });
}


pub fn ui_serverlist(
    mut ctx: EguiContexts, 
    mut cli: EthertiaClient,
    mut edit_i: Local<Option<usize>>,
) { 
    new_egui_window("Server List").show(ctx.ctx_mut(), |ui| {
        let mut serverlist = &mut cli.data().cfg.serverlist;

        let (mut do_new_server, mut do_refresh) = (false, false);

        let mut join_addr = None;
        let mut del_i = None;
        

        ui_lr_panel(ui, false, |ui| {
            if sfx_play(ui.selectable_label(false, "Add Server")).clicked() {
                do_new_server = true;
            }
            if sfx_play(ui.selectable_label(false, "Direct Connect")).clicked() {
                
            }
            if sfx_play(ui.selectable_label(false, "Refresh")).clicked() {
                do_refresh = true;
            }
        }, |ui| {
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
                                ui.label("21ms · 12/64");
                            });
                        }
                    });
                    ui.horizontal(|ui| {
                        if editing {
                            ui.text_edit_singleline(&mut server_item.addr);
                        } else {
                            ui.label(&server_item.addr);
                        }

                        ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                            if editing {
                                if sfx_play(ui.button("✅")).clicked() {
                                    *edit_i = None;
                                }
                            } else {
                                if sfx_play(ui.button("🗑")).on_hover_text("Delete").clicked() {
                                    del_i = Some(idx);
                                }
                                if sfx_play(ui.button("⛭")).on_hover_text("Edit").clicked() {
                                    *edit_i = Some(idx);
                                }
                                if sfx_play(ui.button("⟲")).on_hover_text("Refresh Status").clicked() {
                                    
                                }
                                if sfx_play(ui.button("▶")).on_hover_text("Join & Play").clicked() {
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
            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
        }
        if do_refresh {
            match crate::util::get_server_list("https://ethertia.com/server-info.json") {
                Ok(ret) => *serverlist = ret,
                Err(err) => info!("{}", err),
            }
        }

        if let Some(del_i) = del_i {
            serverlist.remove(del_i);
        }

        if let Some(join_addr) = join_addr {
            // 连接服务器 这两个操作会不会有点松散
            cli.data().curr_ui = CurrentUI::ConnectingServer;
            cli.connect_server(join_addr);
        }
        // if next_ui_1 != CurrentUI::None {
        //     next_ui.set(next_ui_1);
        // }
        // cli.data().cfg.serverlist = serverlist;  // cannot borrow &mut more than once. so copy and assign
    });
}


pub fn ui_localsaves(
    mut ctx: EguiContexts, 
    mut cli: ResMut<ClientInfo>,
) {
    new_egui_window("Local Worlds").show(ctx.ctx_mut(), |ui| {

        ui_lr_panel(ui, false, |ui| {
            if sfx_play(ui.selectable_label(false, "New World")).clicked() {
                cli.curr_ui = CurrentUI::LocalWorldNew;
            }
            if sfx_play(ui.selectable_label(false, "Refresh")).clicked() {
                
            }
        }, |ui| {
            for i in 0..28 {
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
                        ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                            if sfx_play(ui.button("🗑")).on_hover_text("Delete").clicked() {
                            }
                            if sfx_play(ui.button("⛭")).on_hover_text("Edit").clicked() {
                            }
                            if sfx_play(ui.button("▶")).on_hover_text("Play").clicked() {
                            }
                        });
                    });
                });
            }
        });
    });
}

#[derive(Default, Debug, PartialEq)]
pub enum Difficulty {
    Peace,
    #[default]
    Normal,
    Hard,
}

pub fn ui_create_world(
    mut ctx: EguiContexts, 
    mut cli: ResMut<ClientInfo>,
    mut tx_world_name: Local<String>,
    mut tx_world_seed: Local<String>,
    mut _difficulty: Local<Difficulty>,
) {
    new_egui_window("New World").show(ctx.ctx_mut(), |ui| {

        // ui_lr_panel(ui, true, |ui| {
        //     if sfx_play(ui.selectable_label(true, "General")).clicked() {

        //     }
        //     if sfx_play(ui.selectable_label(false, "Generation")).clicked() {
                
        //     }
        //     if sfx_play(ui.selectable_label(false, "Gamerules")).clicked() {
                
        //     }
        // }, |ui| {
            let space = 14.;

            ui.label("Name:");
            sfx_play(ui.text_edit_singleline(&mut *tx_world_name));
            ui.add_space(space);
    
            ui.label("Seed:");
            sfx_play(ui.text_edit_singleline(&mut *tx_world_seed));
            ui.add_space(space);
    
            ui.label("Gamemode:");
            ui.horizontal(|ui| {
                sfx_play(ui.radio_value(&mut *_difficulty, Difficulty::Peace, "Survival"));
                sfx_play(ui.radio_value(&mut *_difficulty, Difficulty::Normal, "Creative"));
                sfx_play(ui.radio_value(&mut *_difficulty, Difficulty::Hard, "Spectator"));
            });
            ui.add_space(space);

    
            ui.label("Difficulty:");
            ui.horizontal(|ui| {
                sfx_play(ui.radio_value(&mut *_difficulty, Difficulty::Peace, "Peace"));
                sfx_play(ui.radio_value(&mut *_difficulty, Difficulty::Normal, "Normal"));
                sfx_play(ui.radio_value(&mut *_difficulty, Difficulty::Hard, "Hard"));
            });
            ui.add_space(space);
    
            ui.label("Difficulty:");
            egui::ComboBox::from_id_source("Difficulty")
                .selected_text(format!("{:?}", *_difficulty))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut *_difficulty, Difficulty::Peace, "Peace");
                    ui.selectable_value(&mut *_difficulty, Difficulty::Normal, "Normal");
                    ui.selectable_value(&mut *_difficulty, Difficulty::Hard, "Hard");
                });

            ui.add_space(space);
    

            ui.add_space(22.);

            if sfx_play(ui.add_sized([290., 26.], egui::Button::new("Create World").fill(Color32::DARK_GREEN))).clicked() {
    
            }
            ui.add_space(4.);
            if sfx_play(ui.add_sized([290., 20.], egui::Button::new("Cancel"))).clicked() {
                cli.curr_ui = CurrentUI::LocalWorldList;
            }
        // });
    });
}