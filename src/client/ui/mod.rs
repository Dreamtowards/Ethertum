mod debug;
pub mod hud;
mod items;
mod main_menu;
pub mod serverlist;
mod settings;

pub mod prelude {
    pub use super::items::{ui_inventory, ui_item_stack};
    pub use super::sfx_play;
    pub use super::CurrentUI;
    pub use super::UiExtra;
    pub use bevy_egui::egui::{self, pos2, vec2, Align2, Color32, InnerResponse, Rect};
    pub use bevy_egui::EguiContexts;
}

use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::fxaa::Fxaa;
use bevy::core_pipeline::Skybox;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::{ScreenSpaceReflections, VolumetricFog};
use bevy_egui::{egui::{
    self, style::HandleShape, Align2, Color32, FontData, FontDefinitions, FontFamily, Layout, Pos2, Response, Rounding, Stroke, Ui, WidgetText,
}, EguiContextSettings, EguiContexts, EguiGlobalSettings, EguiMultipassSchedule, EguiPlugin, EguiPrimaryContextPass, EguiStartupSet, PrimaryEguiContext};
use egui_extras::{Size, StripBuilder};
use rand::Rng;

use crate::client::prelude::*;

pub struct UiPlugin;

#[derive(Default, Resource)]
struct UiState {
    is_window_open: bool,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>();
        app.insert_resource(hud::ChatHistory::default());
        if !app.is_plugin_added::<EguiPlugin>() || true {
            app.add_plugins(EguiPlugin::default());
        }
        
        {
            app
            .add_systems(
                PreStartup,
                setup_camera_system.before(EguiStartupSet::InitContexts),
            )
            .add_systems(
                Startup,
                (configure_visuals_system, configure_ui_state_system),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    /* test */
                    (ui_example_system, update_ui_scale_factor_system),
                    /* debug */
                    debug::ui_menu_panel.run_if(|cli: Res<ClientInfo>| cli.dbg_menubar),
                    debug::hud_debug_text
                        .run_if(|cli: Res<ClientInfo>| cli.dbg_text)
                        .before(debug::ui_menu_panel),
                    /* hud */
                    (hud::hud_hotbar, hud::hud_chat, hud::hud_playerlist.run_if(condition::manipulating)).run_if(condition::in_world),
                    items::draw_ui_holding_item,
                    /* menu */
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
                    )
                ),
            );
        }

        app.add_systems(First, play_bgm);

        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin,
            // SystemInformationDiagnosticsPlugin,
        ));

        /*
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
                FrameTimeDiagnosticsPlugin::default(),
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

            app.add_systems(Update, items::draw_ui_holding_item);
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
        */
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

pub fn color32_of(c: Srgba) -> Color32 {
    Color32::from_rgba_premultiplied((c.red*255.) as u8, (c.green*255.) as u8, (c.blue*255.) as u8, (c.alpha*255.) as u8)
}

pub fn color32_gray_alpha(gray: f32, alpha: f32) -> Color32 {
    let g = (gray * 255.) as u8;
    let a = (alpha * 255.) as u8;
    Color32::from_rgba_premultiplied(g, g, g, a)
}

fn setup_camera_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // WARNING: 不应该产生多个Camera 否则SSR不支持 很多东西也会非预期的绘制多次如gizmos
    // commands.spawn((
    //     Camera2d::default(),
    //     Camera {
    //         order: 10,
    //         hdr: true,  // Sync with Camera3d!
    //         ..default()
    //     }
    // ));

    // NOTE: 也许应该放在通用系统里初始化camera而不是ui里, 但毕竟依赖egui的初始化时序 先暂时放这吧
    let skybox_image = asset_server.load("table_mountain_2_puresky_4k_cubemap.jpg");
    commands.insert_resource(crate::client::client_world::SkyboxCubemap {
        is_loaded: false,
        image_handle: skybox_image.clone()
    });

    // Camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,  // WARNING: Camera3d 和 ui的Camera2d 必须都开启hdr或者都不开启 否则只会渲染order大的那个
            order: 0,
            ..default()
        },
        /*
        bevy::pbr::Atmosphere::EARTH,
        bevy::pbr::AtmosphereSettings {
            aerial_view_lut_max_distance: 3.2e5,
            scene_units_to_m: 1e+4,
            ..Default::default()
        },
        bevy::camera::Exposure::SUNLIGHT,
        bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
        bevy::post_process::bloom::Bloom::NATURAL,
        bevy::light::AtmosphereEnvironmentMapLight::default(),
        */
        // #[cfg(feature = "target_native_os")]
        // bevy_atmosphere::plugin::AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on
        DistanceFog {
            // color, falloff shoud be set in ClientInfo.sky_fog_visibility, etc. due to dynamic debug reason.
            // falloff: FogFalloff::Atmospheric { extinction: Vec3::ZERO, inscattering:  Vec3::ZERO },  // mark as Atmospheric. value will be re-set by ClientInfo.sky_fog...
            ..default()
        },
        Skybox {
            image: skybox_image.clone(),
            brightness: 1000.0,
            ..Default::default()
        },
        EnvironmentMapLight {
            diffuse_map: skybox_image.clone(),
            specular_map: skybox_image.clone(),
            intensity: 1000.0,
            ..Default::default()
        },
        CharacterControllerCamera,
        Name::new("Camera"),
        DespawnOnWorldUnload,

        Msaa::Off,  // Optional 保持原本的设置, 之前关闭Msaa好像是为了SSR?
        // ScreenSpaceReflectionsBundle::default(),
        // Fxaa::default(),
    ))
        .insert(ScreenSpaceReflections::default())  // 会导致渲染异常 3d不渲染 历史没clear UB
        .insert(Fxaa::default())
        .insert(Tonemapping::TonyMcMapface)
        .insert(Bloom::default())
        .insert(VolumetricFog {
            ambient_intensity: 0.,
            //density: 0.01,
            //light_tint: Color::linear_rgb(0.916, 0.941, 1.000),
            ..default()
        })
    ;
}

fn configure_visuals_system(mut contexts: EguiContexts) -> Result {
    /*
    contexts.ctx_mut()?.style_mut(|style| {
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
            blur: 204,
            color: Color32::from_black_alpha(45),
            ..default()
        };
        visuals.popup_shadow = visuals.window_shadow;
    });
    */

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "my_font".to_owned(),
        std::sync::Arc::new(
            FontData::from_static(include_bytes!("../../../assets/fonts/menlo.ttf")),
        ),
    );

    // Put my font first (highest priority):
    fonts.families.get_mut(&FontFamily::Proportional).ok_or(crate::err_opt_is_none!())?.insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts.families.get_mut(&FontFamily::Monospace).ok_or(crate::err_opt_is_none!())?.push("my_font".to_owned());

    contexts.ctx_mut()?.set_fonts(fonts);
    Ok(())
}

fn configure_ui_state_system(mut ui_state: ResMut<UiState>) {
    ui_state.is_window_open = true;
}

fn update_ui_scale_factor_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut toggle_scale_factor: Local<Option<bool>>,
    egui_context: Single<(&mut EguiContextSettings, &Camera)>,
) {
    let (mut egui_settings, camera) = egui_context.into_inner();
    if keyboard_input.just_pressed(KeyCode::Slash) || toggle_scale_factor.is_none() {
        *toggle_scale_factor = Some(!toggle_scale_factor.unwrap_or(true));

        let scale_factor = if toggle_scale_factor.unwrap() {
            1.0
        } else {
            1.0 / camera.target_scaling_factor().unwrap_or(1.0)
        };
        egui_settings.scale_factor = scale_factor;
    }
}

fn ui_example_system(
    mut ui_state: ResMut<UiState>,
    mut is_initialized: Local<bool>,
    mut contexts: EguiContexts,
) -> Result {
    if !*is_initialized {
        *is_initialized = true;
    }
    Ok(())
}

// for SFX
static mut SFX_BTN_HOVERED_ID: egui::Id = egui::Id::NULL;
static mut SFX_BTN_CLICKED: bool = false;

// for ui_panel_lr set curr_ui Back without accessing UI Res
static mut UI_BACK: bool = false;

fn play_bgm(asset_server: Res<AssetServer>, mut cmds: Commands, mut limbo_played: Local<bool>, mut cli: ResMut<ClientInfo>) {
    // if !*limbo_played {
    //     *limbo_played = true;

    //     let ls = [
    //         "sounds/music/limbo.ogg",
    //         "sounds/music/dead_voxel.ogg",
    //         // "sounds/music/milky_way_wishes.ogg",
    //         // "sounds/music/gion.ogg",
    //         "sounds/music/radiance.ogg",
    //     ];

    //     cmds.spawn(AudioBundle {
    //         source: asset_server.load(ls[rand::thread_rng().gen_range(0..ls.len())]),
    //         settings: PlaybackSettings::DESPAWN,
    //     });
    // }

    unsafe {
        static mut LAST_HOVERED_ID: egui::Id = egui::Id::NULL;
        if SFX_BTN_HOVERED_ID != egui::Id::NULL && SFX_BTN_HOVERED_ID != LAST_HOVERED_ID {
            cmds.spawn(
                AudioPlayer::<AudioSource>(asset_server.load("sounds/ui/button.ogg"))
                //.with_settings(PlaybackSettings::DESPAWN),
            );
        }
        LAST_HOVERED_ID = SFX_BTN_HOVERED_ID;
        SFX_BTN_HOVERED_ID = egui::Id::NULL;

        if SFX_BTN_CLICKED {
            cmds.spawn(
                AudioPlayer::<AudioSource>(asset_server.load("sounds/ui/button_large.ogg"))
                //.with_settings(PlaybackSettings::DESPAWN),
            );
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
                strip.cell(|_ui| {});
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

pub trait UiExtra {
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
