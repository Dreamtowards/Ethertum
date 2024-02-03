
use std::collections::VecDeque;

use bevy::{prelude::*, reflect::List};
use bevy_egui::{egui::{self, text::LayoutJob, Align, Align2, Color32, FontId, Frame, ScrollArea, TextEdit, TextFormat}, EguiContexts};
use bevy_renet::renet::RenetClient;

use crate::{game::WorldInfo, net::{CPacket, RenetClientHelper}};

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


    mut net_client: ResMut<RenetClient>,

    input_key: Res<Input<KeyCode>>,
    mut worldinfo: ResMut<WorldInfo>,
    
    mut curr_ui: ResMut<State<CurrentUI>>,
    mut next_ui: ResMut<NextState<CurrentUI>>,
) {
    let has_new_chat = *last_chat_count > state.scrollback.len();
    *last_chat_count = state.scrollback.len();

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
                .auto_shrink([false, false])
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

            if input_key.just_pressed(KeyCode::Slash) && *curr_ui == CurrentUI::None {
                next_ui.set(CurrentUI::ChatInput);
            }
            if *curr_ui != CurrentUI::ChatInput {
                return;
            }

            // ui.separator();

            // Input
            let text_edit = TextEdit::singleline(&mut state.buf)
                .desired_width(f32::INFINITY)
                .lock_focus(true);

            // Handle enter
            let text_edit_response = ui.add(text_edit);

            ui.add_space(5.);
            
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


pub fn hud_hotbar(mut ctx: EguiContexts) {
    egui::Window::new("HUD Hotbar")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::CENTER_BOTTOM, [0., -16.])
        // .frame(Frame::default().fill(Color32::from_black_alpha(180)))
        .show(ctx.ctx_mut(), |ui| {
            let s = 50.;

            ui.horizontal(|ui| {
                for i in 0..9 {
                    ui.add_sized([s, s], egui::Button::new(""));
                }
            });
        });
}