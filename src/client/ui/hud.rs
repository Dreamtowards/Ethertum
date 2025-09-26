use std::collections::VecDeque;

use crate::{client::client_world::ClientPlayerInfo, prelude::*, voxel::{ChunkSystem, ClientChunkSystem, VoxShape, VoxelBrush}};

use bevy_egui::{
    egui::{text::CCursorRange, Align, Frame, Id, Layout, TextEdit, Vec2},
    EguiContexts,
};
use bevy_renet::renet::RenetClient;

use crate::ui::prelude::*;
use crate::{
    client::prelude::*,
    net::{CPacket, RenetClientHelper},
};

use super::{new_egui_window, settings::ui_setting_line};

// todo: Res是什么原理？每次sys调用会deep拷贝吗？还是传递指针？如果deep clone这么多消息记录 估计会很浪费性能。

#[derive(Resource, Default, Debug)]
pub struct ChatHistory {
    pub buf: String,
    pub scrollback: Vec<String>,
    pub history: VecDeque<String>,
    pub history_index: usize,
    // Line prefix symbol
    // pub symbol: String,
    // Number of commands to store in history
    // pub history_size: usize,
}

fn set_cursor_pos(ctx: &egui::Context, id: egui::Id, pos: usize) {
    if let Some(mut state) = TextEdit::load_state(ctx, id) {
        state.cursor.set_char_range(Some(CCursorRange::one(egui::text::CCursor::new(pos))));
        // state.set_ccursor_range(Some(CCursorRange::one(egui::text::CCursor::new(pos))));
        state.store(ctx, id);
    }
}

pub fn hud_chat(
    mut ctx: EguiContexts,
    mut state: ResMut<ChatHistory>,
    mut last_chat_count: Local<usize>,
    mut last_time_new_chat: Local<f32>,
    time: Res<Time>,
    input_key: Res<ButtonInput<KeyCode>>,
    mut cli: ResMut<ClientInfo>, // only curr_ui
    mut net_client: ResMut<RenetClient>,
) {
    let has_new_chat = state.scrollback.len() > *last_chat_count;
    *last_chat_count = state.scrollback.len();

    if input_key.just_pressed(KeyCode::Slash) && cli.curr_ui == CurrentUI::None {
        cli.curr_ui = CurrentUI::ChatInput;
    }

    // Hide ChatUi when long time no new message.
    let curr_time = time.elapsed_secs();
    if has_new_chat {
        *last_time_new_chat = curr_time;
    }
    if *last_time_new_chat < curr_time - 8. && cli.curr_ui != CurrentUI::ChatInput {
        return;
    }

    egui::Window::new("Chat")
        .default_size([620., 320.])
        .title_bar(false)
        .resizable(true)
        .collapsible(false)
        .anchor(Align2::LEFT_BOTTOM, [0., -100.])
        // .frame(Frame::default().fill(Color32::from_black_alpha(140)))
        .show(ctx.ctx_mut().unwrap(), |ui| {
            ui.vertical(|ui| {
                let scroll_height = ui.available_height() - 38.0;

                ui.add_space(4.);

                // Scroll area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .stick_to_bottom(true)
                    .max_height(scroll_height)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for line in &state.scrollback {
                                ui.colored_label(Color32::WHITE, line);
                            }
                        });

                        // Scroll to bottom if have new message
                        // if has_new_chat {
                        //     ui.scroll_to_cursor(Some(Align::BOTTOM));
                        // }
                    });

                // hide input box when gaming.
                if cli.curr_ui != CurrentUI::ChatInput {
                    return;
                }

                // Input
                let text_edit = TextEdit::singleline(&mut state.buf).desired_width(f32::INFINITY).lock_focus(true);

                let text_edit_response = ui.add(text_edit);

                ui.add_space(5.);

                // Handle enter
                if text_edit_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let history_size = 20;

                    // let msg = format!("{}{}", state.symbol, state.buf);
                    // state.scrollback.push(msg.into());
                    let cmdstr = state.buf.clone();

                    if state.history.is_empty() {
                        state.history.push_front(String::default()); // editing line
                    }
                    state.history.insert(1, cmdstr.clone());
                    if state.history.len() > history_size + 1 {
                        state.history.pop_back();
                    }

                    net_client.send_packet(&CPacket::ChatMessage { message: cmdstr.clone() });

                    // let mut args = Shlex::new(&state.buf).collect::<Vec<_>>();

                    // if !args.is_empty() {
                    //     let command_name = args.remove(0);
                    //     debug!("Command entered: `{command_name}`, with args: `{args:?}`");

                    //     let command = config.commands.get(command_name.as_str());

                    //     if command.is_some() {
                    //         command_entered
                    //             .send(ConsoleCommandEntered { command_name, args });
                    //     } else {
                    //         debug!(
                    //             "Command not recognized, recognized commands: `{:?}`",
                    //             config.commands.keys().collect::<Vec<_>>()
                    //         );

                    //         state.scrollback.push("error: Invalid command".into());
                    //     }
                    // }

                    state.buf.clear();

                    // Close ChatUi after Enter.
                    cli.curr_ui = CurrentUI::None;
                }

                // Clear on ctrl+l
                // if keyboard_input_events
                //     .iter()
                //     .any(|&k| k.state.is_pressed() && k.key_code == Some(KeyCode::L))
                //     && (keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]))
                // {
                //     state.scrollback.clear();
                // }

                // Handle up and down through history
                if text_edit_response.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::ArrowUp))
                    && state.history.len() > 1
                    && state.history_index < state.history.len() - 1
                {
                    if state.history_index == 0 && !state.buf.trim().is_empty() {
                        *state.history.get_mut(0).unwrap() = state.buf.clone();
                    }

                    state.history_index += 1;
                    state.buf = state.history.get(state.history_index).unwrap().clone();

                    set_cursor_pos(ui.ctx(), text_edit_response.id, state.buf.len());
                } else if text_edit_response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) && state.history_index > 0 {
                    state.history_index -= 1;
                    state.buf = state.history.get(state.history_index).unwrap().clone();

                    set_cursor_pos(ui.ctx(), text_edit_response.id, state.buf.len());
                }

                // Focus on input
                ui.memory_mut(|m| m.request_focus(text_edit_response.id));
            });
        });
}

pub fn hud_hotbar(mut ctx: EguiContexts, cfg: Res<ClientSettings>, mut player: ResMut<ClientPlayerInfo>,
    mut voxbrush: ResMut<VoxelBrush>,
    // chunk_sys: Res<ClientChunkSystem>,
) {

    // new_egui_window("VoxBrush")
    //     .anchor(Align2::LEFT_BOTTOM, [cfg.hud_padding, -cfg.hud_padding])
    //     .frame(Frame::default().fill(Color32::from_black_alpha(30)))
    //     .show(ctx.ctx_mut(), |ui| {

    //         ui_setting_line(ui, "Size", egui::Slider::new(&mut voxbrush.size, 0.0..=25.0));
    //         ui_setting_line(ui, "Intensity", egui::Slider::new(&mut voxbrush.size, 0.0..=1.0));
    //         ui_setting_line(ui, "Tex", egui::Slider::new(&mut voxbrush.tex, 0..=28));

    //         // ui.painter().image(ctx.add_image(chunk_sys.mtl_terrain), rect, uv, tint)

    //         if ui.btn("Cube").clicked() {
    //             voxbrush.size = 1.;
    //             voxbrush.shape = VoxShape::Cube;
    //         }
    //     });

    egui::Window::new("HUD Hotbar")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::CENTER_BOTTOM, [0., -cfg.hud_padding])
        .frame(Frame::default().fill(Color32::from_black_alpha(0)))
        .show(ctx.ctx_mut().unwrap(), |ui| {
            // Health bar
            {
                let health_bar_size = Vec2::new(250., 4.);
                let mut rect = ui.min_rect();
                rect.set_height(health_bar_size.y);
                rect.set_width(health_bar_size.x);
                let rounding = ui.style().visuals.widgets.inactive.rounding();

                // bar bg
                ui.painter().rect_filled(rect, rounding, Color32::from_black_alpha(200));

                // bar fg
                let rect_fg = rect.with_max_x(rect.min.x + health_bar_size.x * (player.health as f32 / player.health_max as f32));
                ui.painter().rect_filled(rect_fg, rounding, Color32::WHITE);

                // ui.painter().text(rect.left_center(), Align2::LEFT_CENTER,
                //     format!(" {} / {}", cli.health, cli.health_max), FontId::proportional(10.), Color32::BLACK, );

                ui.add_space(health_bar_size.y + 8.);
            }

            ui.horizontal(|ui| {
                for i in 0..ClientPlayerInfo::HOTBAR_SLOTS {
                    let item = player.inventory.items.get_mut(i as usize).unwrap();

                    ui_item_stack(ui, item);
                }
            });
        });
}

pub fn hud_playerlist(
    mut ctx: EguiContexts,
    input_key: Res<ButtonInput<KeyCode>>,
    cli: Res<ClientInfo>,
    cfg: Res<ClientSettings>,
    mut net_client: ResMut<RenetClient>,
) {
    if !input_key.pressed(KeyCode::Tab) {
        return;
    }
    if input_key.just_pressed(KeyCode::Tab) {
        info!("Request PlayerList");
        net_client.send_packet(&CPacket::PlayerList);
    }

    egui::Window::new("PlayerList")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::CENTER_TOP, [0., cfg.hud_padding])
        .show(ctx.ctx_mut().unwrap(), |ui| {
            for player in &cli.playerlist {
                ui.horizontal(|ui| {
                    ui.set_width(280.);

                    // ui.add_sized([180., 24.], egui::Label::new(player.0.as_str()));
                    ui.colored_label(Color32::WHITE, player.0.as_str());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.colored_label(Color32::GRAY, format!("{}ms", player.1));
                    })
                });
            }
            // ui.separator();
            // ui.label("Server MOTD Footer Test");

            // Lock Focus when pressing Tab
            ui.memory_mut(|m| m.request_focus(Id::NULL));
        });
}
