
use std::collections::VecDeque;

use bevy::{prelude::*, reflect::List};
use bevy_egui::{egui::{self, text::LayoutJob, Align, Align2, Color32, FontId, Frame, Id, Layout, Rounding, ScrollArea, Stroke, TextEdit, TextFormat, Vec2}, EguiContexts};
use bevy_renet::renet::RenetClient;
use egui_extras::{Size, StripBuilder};

use crate::{game::{ClientInfo, WorldInfo}, net::{CPacket, RenetClientHelper, SPacket}};

use super::CurrentUI;


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
        state.set_ccursor_range(Some(egui::text_edit::CCursorRange::one(egui::text::CCursor::new(pos))));
        state.store(ctx, id);
    }
}

pub fn hud_chat(
    mut ctx: EguiContexts,
    mut state: ResMut<ChatHistory>,
    mut last_chat_count: Local<usize>,
    mut last_time_new_chat: Local<f32>,
    time: Res<Time>,

    mut net_client: ResMut<RenetClient>,

    input_key: Res<Input<KeyCode>>,
    mut curr_ui: ResMut<State<CurrentUI>>,
    mut next_ui: ResMut<NextState<CurrentUI>>,
) {
    let has_new_chat = state.scrollback.len() > *last_chat_count;
    *last_chat_count = state.scrollback.len();

    if input_key.just_pressed(KeyCode::Slash) && *curr_ui == CurrentUI::None {
        next_ui.set(CurrentUI::ChatInput);
    }

    // Hide ChatUi when long time no new message.
    let curr_time = time.elapsed_seconds();
    if has_new_chat {
        *last_time_new_chat = curr_time;
    } 
    if *last_time_new_chat < curr_time - 8. && *curr_ui != CurrentUI::ChatInput {
        return;  
    }

    egui::Window::new("Chat")
    .default_size([620., 320.])
    .title_bar(false)
    .resizable(true)
    .collapsible(false)
    .anchor(Align2::LEFT_BOTTOM, [0., -100.])
    // .frame(Frame::default().fill(Color32::from_black_alpha(140)))
    .show(ctx.ctx_mut(), |ui| {
        ui.vertical(|ui| {
            let scroll_height = ui.available_height() - 38.0;

            ui.add_space(4.);

            // Scroll area
            ScrollArea::vertical()
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
            if *curr_ui != CurrentUI::ChatInput {
                return;
            }

            // Input
            let text_edit = TextEdit::singleline(&mut state.buf)
                .desired_width(f32::INFINITY)
                .lock_focus(true);

            let text_edit_response = ui.add(text_edit);

            ui.add_space(5.);
            
            // Handle enter
            if text_edit_response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                let history_size = 20;

                // let msg = format!("{}{}", state.symbol, state.buf);
                // state.scrollback.push(msg.into());
                let cmdstr = state.buf.clone();

                if state.history.len() == 0 {
                    state.history.push_front(String::default());  // editing line 
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
                next_ui.set(CurrentUI::None);
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
                    *state.history.get_mut(0).unwrap() = state.buf.clone().into();
                }

                state.history_index += 1;
                state.buf = state.history.get(state.history_index).unwrap().clone();

                set_cursor_pos(ui.ctx(), text_edit_response.id, state.buf.len());
            } else if text_edit_response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::ArrowDown))
                && state.history_index > 0
            {
                state.history_index -= 1;
                state.buf = state.history.get(state.history_index).unwrap().clone();

                set_cursor_pos(ui.ctx(), text_edit_response.id, state.buf.len());
            }

            // Focus on input
            ui.memory_mut(|m| m.request_focus(text_edit_response.id));
        });
    });
}


pub fn hud_hotbar(
    mut ctx: EguiContexts,
    cli: Res<ClientInfo>,

) {
    egui::Window::new("HUD Hotbar")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::CENTER_BOTTOM, [0., -cli.cfg.hud_padding])
        .frame(Frame::default().fill(Color32::from_black_alpha(0)))
        .show(ctx.ctx_mut(), |ui| {
            let s = 50.;

            let healthbar_size = Vec2::new(250., 10.);
            let mut min = ui.min_rect();
            min.set_height(0.);
            min.set_width(0.);
            min = min.translate(Vec2::new(0., -healthbar_size.y - 10.));
            
            ui.painter().rect_filled(
                min.expand2(healthbar_size), 
                Rounding::ZERO,
                Color32::from_black_alpha(200),
            );

            ui.painter().rect_filled(
                min.expand2(Vec2::new(healthbar_size.x * (cli.health as f32 / cli.health_max as f32), healthbar_size.y)), 
                Rounding::ZERO,
                Color32::RED,
            );
            ui.painter().text(
                min.left_center(), Align2::LEFT_CENTER, 
                format!("{}/{}", cli.health, cli.health_max), 
                FontId::proportional(12.), Color32::WHITE
            );



            ui.horizontal(|ui| {
                for i in 0..crate::game::HOTBAR_SLOTS {
                    let mut slot = egui::Button::new("").fill(Color32::from_black_alpha(100));

                    if cli.hotbar_index == i {
                        slot = slot.stroke(Stroke::new(3., Color32::WHITE));
                    }

                    ui.add_sized([s, s], slot);
                }
            });

        });
        
}

pub fn hud_playerlist(
    mut ctx: EguiContexts,
    input_key: Res<Input<KeyCode>>,
    
    cli: Res<ClientInfo>,
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
        .anchor(Align2::CENTER_TOP, [0., cli.cfg.hud_padding])
        .show(ctx.ctx_mut(), |ui| {

            for player in &cli.playerlist {
                
                ui.horizontal(|ui| {
                    ui.set_width(280.);

                    // ui.add_sized([180., 24.], egui::Label::new(player.0.as_str()));
                    ui.colored_label(Color32::WHITE, player.0.as_str());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.colored_label(Color32::GREEN, format!("{}ms", player.1));
                    })
                });
            }
            // ui.separator();
            // ui.label("Server MOTD Footer Test");
            
            // Lock Focus when pressing Tab
            ui.memory_mut(|m| m.request_focus(Id::NULL));
        });
}