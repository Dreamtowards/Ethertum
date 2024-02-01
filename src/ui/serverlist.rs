
use bevy_egui::{egui::{self, Align2, Color32, Layout}, EguiContexts};

pub fn ui_serverlist(mut ctx: EguiContexts) {
    egui::Window::new("ServerList").title_bar(false).anchor(Align2::CENTER_CENTER, [0., 0.]).resizable(true).collapsible(false).show(ctx.ctx_mut(), |ui| {

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let btnsize = [160., 22.];
                if ui.add_sized(btnsize, egui::Button::new("Join Server")).clicked() {
                }
                if ui.add_sized(btnsize, egui::Button::new("Add Server")).clicked() {
                }
                if ui.add_sized(btnsize, egui::Button::new("Direct Connect")).clicked() {
                }
                if ui.add_sized(btnsize, egui::Button::new("Refresh")).clicked() {
                }
                if ui.add_sized(btnsize, egui::Button::new("Cancel")).clicked() {
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