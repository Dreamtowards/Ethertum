use bevy::{prelude::*, app::AppExit, ecs::{event::EventWriter, system::Commands}};
use bevy_egui::{
    egui::{self, pos2, Align2, Color32, Frame, Layout, OpenUrl, RichText, Rounding },
    EguiContexts, 
};
use bevy_renet::renet::{transport::NetcodeClientTransport, RenetClient};

use crate::game_client::{ClientInfo, EthertiaClient, WorldInfo};
use super::{CurrentUI, UiExtra};


pub fn ui_main_menu(
    // mut rendered_texture_id: Local<egui::TextureId>,
    // asset_server: Res<AssetServer>,
    mut app_exit_events: EventWriter<AppExit>,
    mut ctx: EguiContexts,
    mut cli: EthertiaClient,
    mut cmds: Commands,

    mut next_ui: ResMut<NextState<CurrentUI>>,

    // mut dbg_server_addr: Local<String>,
) {
    // if *rendered_texture_id == egui::TextureId::default() {
    //     *rendered_texture_id = ctx.add_image(asset_server.load("ui/main_menu/1.png"));
    // }

    egui::CentralPanel::default().show(ctx.ctx_mut(), |ui| {
        let h = ui.available_height();

        // ui.painter().image(*rendered_texture_id, ui.max_rect(), Rect::from_min_max([0.0, 0.0].into(), [1.0, 1.0].into()), Color32::WHITE);

        ui.vertical_centered(|ui| {
            ui.add_space(h * 0.16);
            ui.add(egui::Label::new(RichText::new("ethertia").heading().color(Color32::WHITE)));
            ui.add_space(h * 0.24);

            // if dbg_server_addr.is_empty() {
            //     *dbg_server_addr = "127.0.0.1:4000".into();
            // }
            // ui.add_sized(siz, egui::TextEdit::singleline(&mut *dbg_server_addr));
            // if ui.add_sized(siz, egui::Button::new("Connect to Server")).clicked() {
            //     // 连接服务器 这两个操作会不会有点松散
            //     next_ui.set(CurrentUI::ConnectingServer);
            //     cli.connect_server(dbg_server_addr.clone());
            // }
            // // if ui.add_sized(siz, egui::Button::new("Debug Local")).clicked() {
            // //     // 临时的单人版方法 直接进入世界而不管网络
            // //     next_ui.set(CurrentUI::None);
            // //     commands.insert_resource(WorldInfo::default());  
            // // }
            // ui.label("·");

            // if ui.add_sized(siz, egui::Button::new("Singleplayer")).clicked() {
            //     next_ui.set(CurrentUI::LocalSaves);
            // }
            if ui.btn_normal("Singleplayer").clicked() {
                next_ui.set(CurrentUI::LocalSaves);
            }
            if ui.btn_normal("Multiplayer").clicked() {
                next_ui.set(CurrentUI::WtfServerList);
            }
            if ui.btn_normal("Settings").clicked() {
                next_ui.set(CurrentUI::WtfSettings);
            }
            if ui.btn_normal("Terminate").clicked() {
                app_exit_events.send(AppExit);
            }
        });

        ui.with_layout(Layout::bottom_up(egui::Align::RIGHT), |ui| {
            ui.label("Copyright © nil. Do distribute!");
        });

        ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(false, "").on_hover_text("Github Repository").clicked() {
                    ui.ctx().open_url(OpenUrl::new_tab("https://github.com/Dreamtowards/Ethertum"));
                }  
                if ui.selectable_label(false, "").on_hover_text("Steam").clicked() {
                    ui.ctx().open_url(OpenUrl::new_tab("https://github.com/Dreamtowards/Ethertum"));
                }
                if ui.selectable_label(false, "").on_hover_text("YouTube").clicked() {
                    ui.ctx().open_url(OpenUrl::new_tab("https://github.com/Dreamtowards/Ethertum"));
                }
                if ui.selectable_label(false, "⛓").on_hover_text("Wiki & Documentations").clicked() {
                    ui.ctx().open_url(OpenUrl::new_tab("https://docs.ethertia.com"));
                }
                ui.label("|");
                ui.selectable_label(false, "");  // Windows
                ui.selectable_label(false, "🐧");
                ui.selectable_label(false, "");
                ui.selectable_label(false, "");  // Android
                ui.label("·");
                // ui.selectable_label(false, "");  // Texture
                ui.selectable_label(false, "⛶");
                ui.selectable_label(false, "⛭");
                ui.selectable_label(false, "🖴");  // Disk
                // ui.selectable_label(false, "☢");
                ui.selectable_label(false, "⎆");
            });
            ui.label(format!("v{}\n0 mods loaded.", std::env!("CARGO_PKG_VERSION")));
        });
    });
}






pub fn ui_pause_menu(
    mut ctx: EguiContexts,
    mut commands: Commands,
    mut next_ui: ResMut<NextState<CurrentUI>>,

    mut net_client: ResMut<RenetClient>,
    // mut worldinfo: ResMut<WorldInfo>,
) {
    // egui::Window::new("Pause Menu").show(ctx.ctx_mut(), |ui| {
    egui::CentralPanel::default()
        .frame(Frame::default().fill(Color32::from_black_alpha(190)))
        .show(ctx.ctx_mut(), |ui| {
            let w = ui.available_width();

            let head_y = 75.;
            ui.painter().rect_filled(
                ui.max_rect().with_max_y(head_y),
                Rounding::ZERO,
                Color32::from_rgba_premultiplied(35, 35, 35, 210),
            );
            ui.painter().rect_filled(
                ui.max_rect().with_max_y(head_y).with_min_y(head_y - 2.),
                Rounding::ZERO,
                Color32::from_white_alpha(80),
            );

            ui.add_space(head_y - 27.);

            ui.horizontal(|ui| {
                ui.add_space((w - 420.) / 2.);

                ui.style_mut().spacing.button_padding.x = 10.;

                ui.toggle_value(&mut false, "Map");
                ui.toggle_value(&mut false, "Inventory");
                ui.toggle_value(&mut false, "Team");
                ui.toggle_value(&mut false, "Abilities");
                ui.toggle_value(&mut false, "Quests");
                ui.separator();
                if ui.toggle_value(&mut false, "Settings").clicked() {
                    next_ui.set(CurrentUI::WtfSettings);
                }
                if ui.toggle_value(&mut false, "Quit").clicked() {
                    next_ui.set(CurrentUI::MainMenu);
                    commands.remove_resource::<WorldInfo>();
                    net_client.disconnect();
                    // cli.close_world();
                }
            });

            // let h = ui.available_height();
            // ui.add_space(h * 0.2);

            // ui.vertical_centered(|ui| {

            //     if ui.add_sized([200., 20.], egui::Button::new("Continue")).clicked() {
            //         next_state_ingame.set(GameInput::Controlling);
            //     }
            //     if ui.add_sized([200., 20.], egui::Button::new("Back to Title")).clicked() {
            //     }
            // });
        });
}
