
use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin}, math::vec2, prelude::*, transform::commands
};
use bevy_egui::{
    egui::{
        self, pos2, style::HandleShape, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, LayerId, Layout, Rangef, Rect,
        Rounding, Stroke, Ui, Widget,
    },
    EguiContexts, EguiPlugin, EguiSettings,
};
use egui_extras::{Size, StripBuilder};
use bevy_common_assets::json::JsonAssetPlugin;

use crate::{
    game_client::{condition, ClientInfo},
};

mod serverlist;
mod main_menu;

#[cfg(feature = "target_native_os")]
mod debug;

mod settings;
pub mod hud;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        // Setup Egui Style
        app.add_systems(Startup, setup_egui_style);

        // Debug UI
        #[cfg(feature = "target_native_os")]
        {
            app.add_systems(Update, debug::ui_menu_panel.run_if(|cli: Res<ClientInfo>| cli.dbg_menubar)); // Debug MenuBar. before CentralPanel
            app.add_systems(Update, debug::hud_debug_text.run_if(|cli: Res<ClientInfo>| cli.dbg_text).before(debug::ui_menu_panel));
    
            app.add_plugins((
                FrameTimeDiagnosticsPlugin,
                EntityCountDiagnosticsPlugin,
                // SystemInformationDiagnosticsPlugin,
            ));
        }

        // HUDs
        {
            app.add_systems(Update, (
                hud::hud_hotbar,
                hud::hud_chat,
                hud::hud_playerlist.run_if(condition::manipulating()),
            ).run_if(condition::in_world()));
            
            app.insert_resource(hud::ChatHistory::default());
        }
        
        app.add_state::<CurrentUI>();

        app.add_systems(Update, 
            (
                settings::ui_settings.run_if(in_state(CurrentUI::WtfSettings)),
                main_menu::ui_pause_menu.run_if(in_state(CurrentUI::PauseMenu)),

                // Menus
                main_menu::ui_main_menu.run_if(in_state(CurrentUI::MainMenu)),
                serverlist::ui_localsaves.run_if(in_state(CurrentUI::LocalSaves)),

                serverlist::ui_serverlist.run_if(in_state(CurrentUI::WtfServerList)),
                serverlist::ui_connecting_server.run_if(in_state(CurrentUI::ConnectingServer)),
                serverlist::ui_disconnected_reason.run_if(in_state(CurrentUI::DisconnectedReason)),
            )
            //.chain()
            //.before(debug::ui_menu_panel)
        );


    }
}


#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, States)]
pub enum CurrentUI {
    None,
    #[default]
    MainMenu,
    PauseMenu,
    WtfSettings,
    WtfServerList,
    ConnectingServer,
    DisconnectedReason,
    ChatInput,
    LocalSaves,
}



pub fn new_egui_window(title: &str) -> egui::Window {
    egui::Window::new(title)
        .default_size([680., 420.])
        .title_bar(false) 
        .anchor(Align2::CENTER_CENTER, [0., 0.])
        .resizable(false)
        .collapsible(false)
}

pub fn color32_of(c: Color) -> Color32 {
    let c = c.as_rgba_u8();
    Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}

pub fn color32_gray_alpha(gray: f32, alpha: f32) -> Color32 {
    let g = (gray * 255.) as u8;
    let a = (alpha * 255.) as u8;
    Color32::from_rgba_premultiplied(g, g, g, a)
}

fn setup_egui_style(mut egui_settings: ResMut<EguiSettings>, mut ctx: EguiContexts) {
    ctx.ctx_mut().style_mut(|style| {
        let mut visuals = &mut style.visuals;
        let round = Rounding::from(2.);

        visuals.window_rounding = round;
        visuals.widgets.noninteractive.rounding = round;
        visuals.widgets.inactive.rounding = round;
        visuals.widgets.hovered.rounding = round;
        visuals.widgets.active.rounding = round;
        visuals.widgets.open.rounding = round;
        visuals.window_rounding = round;
        visuals.menu_rounding = round;

        visuals.collapsing_header_frame = true;
        visuals.handle_shape = HandleShape::Rect { aspect_ratio: 0.5 };
        visuals.slider_trailing_fill = true;

        visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, Color32::from_white_alpha(180));
        visuals.widgets.active.bg_stroke = Stroke::new(3.0, Color32::WHITE);

        visuals.widgets.inactive.weak_bg_fill = Color32::from_white_alpha(10);  // button
        visuals.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(20); // button hovered
        visuals.widgets.active.weak_bg_fill = Color32::from_white_alpha(60); // button pressed

        visuals.selection.bg_fill = Color32::from_rgb(27,76,201);
        visuals.selection.stroke = Stroke::new(2.0, color32_gray_alpha(1., 0.78));  // visuals.selection.bg_fill

        visuals.extreme_bg_color = color32_gray_alpha(0.02, 0.66);  // TextEdit, ProgressBar, ScrollBar Bg, Plot Bg

        visuals.window_fill = color32_gray_alpha(0.1, 0.99);
        visuals.window_shadow = egui::epaint::Shadow{ extrusion: 8., color: Color32::from_black_alpha(45) };
        visuals.popup_shadow = visuals.window_shadow;
    });

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "my_font".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/menlo.ttf")),
    );

    // Put my font first (highest priority):
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts.families.get_mut(&FontFamily::Monospace).unwrap().push("my_font".to_owned());

    ctx.ctx_mut().set_fonts(fonts);

    // egui_settings.scale_factor = 1.;
}





pub fn ui_lr_panel(
    ui: &mut Ui,
    separator: bool,
    mut add_nav: impl FnMut(&mut Ui),
    next_ui: &mut ResMut<NextState<CurrentUI>>,
    mut add_main: impl FnMut(&mut Ui),
) {
    
    let mut builder = StripBuilder::new(ui)
    .size(Size::exact(120.0));  // Left 
    if separator {
        builder = builder.size(Size::exact(0.0));  // Separator
    }
    builder
    .size(Size::remainder().at_least(300.0)) // Right
    .horizontal(|mut strip| {
        strip.strip(|builder| {
            builder
                .size(Size::remainder())
                .size(Size::exact(40.))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        ui.add_space(8.);
                        ui.style_mut().spacing.item_spacing.y = 7.;
                        ui.style_mut().spacing.button_padding.y = 3.;
                        
                        ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                            add_nav(ui);
                        });
                    });
                    strip.cell(|ui| {
                        ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
                            if ui.selectable_label(false, "⬅Back").clicked() {
                                next_ui.set(CurrentUI::MainMenu);
                            }
                        });
                    });
                });
        });
        if separator {
            strip.cell(|ui| {});
        }
        strip.cell(|ui| {
            if separator {
                let p = ui.cursor().left_top() + egui::Vec2::new(-ui.style().spacing.item_spacing.x, 0.);
                let p2 = egui::pos2(p.x, p.y+ui.available_height());
                ui.painter().line_segment([p, p2], ui.visuals().widgets.noninteractive.bg_stroke);
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                add_main(ui);
            });
        });
    });
}
