use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{
    egui::{
        self, style::HandleShape, Align2, Color32, FontData, FontDefinitions, FontFamily, Layout, Pos2, Response, Rounding, Stroke, Ui, WidgetText,
    },
    EguiContexts, EguiPlugin,
};
use egui_extras::{Size, StripBuilder};

use crate::game_client::{condition, ClientInfo};

mod debug;
pub mod hud;
mod main_menu;
pub mod serverlist;
mod settings;


pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        // Setup Egui Style
        app.add_systems(Startup, setup_egui_style);

        app.add_systems(First, play_bgm);

        // Debug UI
        {
            app.add_systems(Update, debug::ui_menu_panel.run_if(|cli: Res<ClientInfo>| cli.dbg_menubar)); // Debug MenuBar. before CentralPanel
            app.add_systems(
                Update,
                debug::hud_debug_text
                    .run_if(|cli: Res<ClientInfo>| cli.dbg_text)
                    .before(debug::ui_menu_panel),
            );

            app.add_plugins((
                FrameTimeDiagnosticsPlugin,
                EntityCountDiagnosticsPlugin,
                // SystemInformationDiagnosticsPlugin,
            ));
        }

        // HUDs
        {
            app.add_systems(
                Update,
                (hud::hud_hotbar, hud::hud_chat, hud::hud_playerlist.run_if(condition::manipulating)).run_if(condition::in_world),
            );

            app.insert_resource(hud::ChatHistory::default());
        }

        app.add_systems(
            Update,
            (
                settings::ui_settings.run_if(condition::in_ui(CurrentUI::Settings)),
                main_menu::ui_pause_menu.run_if(condition::in_ui(CurrentUI::PauseMenu)),
                // Menus
                main_menu::ui_main_menu.run_if(condition::in_ui(CurrentUI::MainMenu)),
                serverlist::ui_localsaves.run_if(condition::in_ui(CurrentUI::LocalWorldList)),
                serverlist::ui_create_world.run_if(condition::in_ui(CurrentUI::LocalWorldNew)),
                serverlist::ui_serverlist.run_if(condition::in_ui(CurrentUI::ServerList)),
                serverlist::ui_connecting_server.run_if(condition::in_ui(CurrentUI::ConnectingServer)),
                serverlist::ui_disconnected_reason.run_if(condition::in_ui(CurrentUI::DisconnectedReason)),
            ), //.chain()
               //.before(debug::ui_menu_panel)
        );
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub enum CurrentUI {
    None,
    #[default]
    MainMenu,
    PauseMenu,
    Settings,
    ServerList,
    ConnectingServer,
    DisconnectedReason,
    ChatInput,
    LocalWorldList,
    LocalWorldNew,
}

// for fn new_egui_window
pub static mut _WINDOW_SIZE: Vec2 = Vec2::ZERO;

pub fn new_egui_window(title: &str) -> egui::Window {
    let size = [680., 420.];

    let mut w = egui::Window::new(title)
        .default_size(size)
        .resizable(true)
        .title_bar(false)
        .anchor(Align2::CENTER_CENTER, [0., 0.])
        .collapsible(false);

    let window_size = unsafe { _WINDOW_SIZE };
    if window_size.x - size[0] < 100. || window_size.y - size[1] < 100. {
        w = w.fixed_size([window_size.x - 12., window_size.y - 12.]).resizable(false);
    }

    w
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

fn setup_egui_style(mut ctx: EguiContexts) {
    ctx.ctx_mut().style_mut(|style| {
        let visuals = &mut style.visuals;
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

        visuals.widgets.inactive.weak_bg_fill = Color32::from_white_alpha(10); // button
        visuals.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(20); // button hovered
        visuals.widgets.active.weak_bg_fill = Color32::from_white_alpha(60); // button pressed

        visuals.selection.bg_fill = Color32::from_rgb(27, 76, 201);
        visuals.selection.stroke = Stroke::new(2.0, color32_gray_alpha(1., 0.78)); // visuals.selection.bg_fill

        visuals.extreme_bg_color = color32_gray_alpha(0.02, 0.66); // TextEdit, ProgressBar, ScrollBar Bg, Plot Bg

        visuals.window_fill = color32_gray_alpha(0.1, 0.99);
        visuals.window_shadow = egui::epaint::Shadow {
            extrusion: 8.,
            color: Color32::from_black_alpha(45),
        };
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
}

// for SFX
static mut SFX_BTN_HOVERED_ID: egui::Id = egui::Id::NULL;
static mut SFX_BTN_CLICKED: bool = false;

// for ui_panel_lr set curr_ui Back without accessing UI Res
static mut UI_BACK: bool = false;

fn play_bgm(asset_server: Res<AssetServer>, mut cmds: Commands, mut limbo_played: Local<bool>, mut cli: ResMut<ClientInfo>) {
    if !*limbo_played {
        *limbo_played = true;

        let ls = [
            "sounds/music/limbo.ogg",
            "sounds/music/dead_voxel.ogg",
            "sounds/music/milky_way_wishes.ogg",
            "sounds/music/gion.ogg",
        ];

        cmds.spawn(AudioBundle {
            source: asset_server.load(ls[crate::util::current_timestamp_millis() as usize % ls.len()]),
            settings: PlaybackSettings::DESPAWN,
        });
    }

    unsafe {
        static mut LAST_HOVERED_ID: egui::Id = egui::Id::NULL;
        if SFX_BTN_HOVERED_ID != egui::Id::NULL && SFX_BTN_HOVERED_ID != LAST_HOVERED_ID {
            cmds.spawn(AudioBundle {
                source: asset_server.load("sounds/ui/button.ogg"),
                settings: PlaybackSettings::DESPAWN,
            });
        }
        LAST_HOVERED_ID = SFX_BTN_HOVERED_ID;
        SFX_BTN_HOVERED_ID = egui::Id::NULL;

        if SFX_BTN_CLICKED {
            cmds.spawn(AudioBundle {
                source: asset_server.load("sounds/ui/button_large.ogg"),
                settings: PlaybackSettings::DESPAWN,
            });
        }
        SFX_BTN_CLICKED = false;

        if UI_BACK {
            UI_BACK = false;
            cli.curr_ui = CurrentUI::MainMenu;
        }
    }
}

// UI Panel: Left-Navs and Right-Content
pub fn ui_lr_panel(ui: &mut Ui, separator: bool, mut add_nav: impl FnMut(&mut Ui), mut add_main: impl FnMut(&mut Ui)) {
    let mut builder = StripBuilder::new(ui).size(Size::exact(120.0)); // Left
    if separator {
        builder = builder.size(Size::exact(0.0)); // Separator
    }
    builder
        .size(Size::remainder().at_least(300.0)) // Right
        .horizontal(|mut strip| {
            strip.strip(|builder| {
                builder.size(Size::remainder()).size(Size::exact(40.)).vertical(|mut strip| {
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
                            if sfx_play(ui.selectable_label(false, "⬅Back")).clicked() {
                                unsafe {
                                    UI_BACK = true;
                                }
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
                    let p2 = Pos2::new(p.x, p.y + ui.available_height());
                    ui.painter().line_segment([p, p2], ui.visuals().widgets.noninteractive.bg_stroke);
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    add_main(ui);
                });
            });
        });
}

trait UiExtra {
    fn btn(&mut self, text: impl Into<WidgetText>) -> Response;

    fn btn_normal(&mut self, text: impl Into<WidgetText>) -> Response;

    fn btn_borderless(&mut self, text: impl Into<WidgetText>) -> Response;
}

pub fn sfx_play(resp: Response) -> Response {
    if resp.hovered() || resp.gained_focus() {
        unsafe {
            SFX_BTN_HOVERED_ID = resp.id;
        }
    }
    if resp.clicked() {
        unsafe {
            SFX_BTN_CLICKED = true;
        }
    }
    resp
}

impl UiExtra for Ui {
    fn btn(&mut self, text: impl Into<WidgetText>) -> Response {
        sfx_play(self.add(egui::Button::new(text)))
    }
    fn btn_normal(&mut self, text: impl Into<WidgetText>) -> Response {
        self.add_space(4.);
        sfx_play(self.add_sized([220., 24.], egui::Button::new(text)))
    }
    fn btn_borderless(&mut self, text: impl Into<WidgetText>) -> Response {
        sfx_play(self.add(egui::SelectableLabel::new(false, text)))
    }
}
