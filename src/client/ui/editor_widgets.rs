//! 独立的 Feathers 控件测试页，由 egui View 菜单按需开启。
//! 主菜单仍由 main_menu.rs 绘制，测试页离场时销毁原生 UI 与焦点。
use super::CurrentUI;
use crate::client::prelude::ClientInfo;
use bevy::{
    feathers::{
        containers::{pane, pane_body, pane_header},
        controls::FeathersButton,
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens, FeathersPlugins,
    },
    input_focus::{tab_navigation::TabGroup, AutoFocus, InputFocus},
    prelude::*,
    ui_widgets::{Activate, ControlOrientation, ScrollArea},
    window::PrimaryWindow,
};
use bevy_egui::{egui, EguiInput, EguiPreUpdateSet, PrimaryEguiContext};

pub(super) struct EditorWidgetsPlugin;
impl Plugin for EditorWidgetsPlugin {
    fn build(&self, app: &mut App) {
        super::widget_gallery::configure(app);
        app.add_plugins(FeathersPlugins)
            .insert_resource(UiTheme(create_dark_theme()))
            .init_resource::<GallerySession>()
            .add_systems(
                PreUpdate,
                reserve_gallery_navigation
                    .after(EguiPreUpdateSet::ProcessInput)
                    .before(EguiPreUpdateSet::BeginPass),
            )
            .add_systems(Update, sync_gallery);
    }
}

#[derive(Resource, Default)]
pub(crate) struct GallerySession {
    previous: Option<CurrentUI>,
}
impl GallerySession {
    pub(super) fn open(&mut self, cli: &mut ClientInfo) {
        if cli.curr_ui != CurrentUI::WidgetGallery {
            self.previous = Some(cli.curr_ui.clone());
            cli.curr_ui = CurrentUI::WidgetGallery;
        }
    }
    pub(crate) fn close(&mut self, cli: &mut ClientInfo) {
        // 页面可能已被断线、退出世界或其他 egui 操作切走，旧按钮不能覆盖新页面。
        if cli.curr_ui == CurrentUI::WidgetGallery {
            cli.curr_ui = self.previous.take().unwrap_or(CurrentUI::MainMenu);
        } else {
            self.previous = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ClientInfo>()
            .init_resource::<GallerySession>()
            .init_resource::<UiScale>()
            .init_resource::<InputFocus>()
            .add_systems(Update, sync_gallery);
        app
    }

    #[test]
    fn gallery_is_closed_by_default_and_does_not_spawn_on_egui_pages() {
        let mut app = app();
        for page in [CurrentUI::MainMenu, CurrentUI::Settings, CurrentUI::ServerList] {
            app.world_mut().resource_mut::<ClientInfo>().curr_ui = page;
            app.update();
            assert_eq!(app.world_mut().query::<&GalleryRoot>().iter(app.world()).count(), 0);
            assert_eq!(app.world().resource::<UiScale>().0, 1.0);
        }
    }

    #[test]
    fn closing_gallery_restores_previous_page_and_cleans_up_focus() {
        for previous in [CurrentUI::MainMenu, CurrentUI::Settings, CurrentUI::None] {
            let mut app = app();
            app.world_mut().resource_mut::<ClientInfo>().curr_ui = previous.clone();
            let mut session = GallerySession::default();
            session.open(&mut app.world_mut().resource_mut::<ClientInfo>());
            // 再次开启不能把返回页面替换成测试页本身。
            session.open(&mut app.world_mut().resource_mut::<ClientInfo>());
            app.insert_resource(session);
            let root = app.world_mut().spawn(GalleryRoot).id();
            let close = app.world_mut().spawn(ChildOf(root)).observe(close_gallery).id();
            app.insert_resource(InputFocus::from_entity(close));
            app.update();
            app.update();
            app.world_mut().trigger(Activate { entity: close });
            assert_eq!(app.world().resource::<ClientInfo>().curr_ui, previous);
            app.update();
            assert!(app.world().get_entity(root).is_err());
            assert!(app.world().get_entity(close).is_err());
            assert!(app.world().resource::<InputFocus>().get().is_none());
            assert_eq!(app.world().resource::<UiScale>().0, 1.0);
        }
    }

    #[test]
    fn stale_close_does_not_override_an_external_page_change() {
        let mut cli = ClientInfo::default();
        let mut session = GallerySession::default();
        session.open(&mut cli);
        cli.curr_ui = CurrentUI::DisconnectedReason;
        session.close(&mut cli);
        assert_eq!(cli.curr_ui, CurrentUI::DisconnectedReason);
        assert!(session.previous.is_none());
    }

    #[test]
    fn navigation_filter_is_only_active_on_gallery() {
        let mut world = World::new();
        world.init_resource::<ClientInfo>();
        let events = vec![
            egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            },
            egui::Event::PointerMoved(egui::pos2(12.0, 12.0)),
        ];
        let entity = world
            .spawn((
                EguiInput(egui::RawInput {
                    events: events.clone(),
                    ..default()
                }),
                PrimaryEguiContext,
            ))
            .id();
        world.run_system_once(reserve_gallery_navigation).unwrap();
        assert_eq!(world.get::<EguiInput>(entity).unwrap().events, events);
        world.resource_mut::<ClientInfo>().curr_ui = CurrentUI::WidgetGallery;
        world.run_system_once(reserve_gallery_navigation).unwrap();
        assert_eq!(world.get::<EguiInput>(entity).unwrap().events, events[1..]);
    }
}

#[derive(Component, Default, Clone)]
struct GalleryRoot;

fn close_gallery(_event: On<Activate>, mut cli: ResMut<ClientInfo>, mut session: ResMut<GallerySession>) {
    session.close(&mut cli);
}

fn reserve_gallery_navigation(cli: Res<ClientInfo>, mut inputs: Query<&mut EguiInput, With<PrimaryEguiContext>>) {
    if cli.curr_ui != CurrentUI::WidgetGallery {
        return;
    }
    // 只过滤测试页：恢复后的 egui 主菜单与其他页面保持原有键盘输入。
    for mut input in &mut inputs {
        input.events.retain(|event| {
            !matches!(
                event,
                egui::Event::Key {
                    key: egui::Key::Tab | egui::Key::Enter | egui::Key::Space,
                    ..
                }
            )
        });
    }
}

fn gallery_pane(caption: &'static str, content: impl SceneList) -> impl Scene {
    bsn! {
        pane()
        Node { flex_grow: 1.0, flex_basis: px(0), min_width: px(240), height: percent(100), min_height: px(0) }
        Children [
            (pane_header() Children [(Text(caption) ThemedText)]),
            (
                pane_body()
                Node { flex_grow: 1.0, min_height: px(0), padding: UiRect::right(px(12)) }
                Children [
                    (
                        #viewport
                        Name(caption)
                        Node { height: percent(100), min_height: px(0), flex_direction: FlexDirection::Column, overflow: Overflow::scroll_y() }
                        ScrollArea
                        Children [{content}]
                    ),
                    (
                        @bevy::feathers::controls::FeathersScrollbar { @target: #viewport, @orientation: ControlOrientation::Vertical }
                        Node { position_type: PositionType::Absolute, right: px(3), top: px(3), bottom: px(3), width: px(6) }
                    ),
                ]
            ),
        ]
    }
}

fn gallery_scene() -> impl Scene {
    bsn! {
        GalleryRoot
        Name("Widget gallery")
        TabGroup { order: 0, modal: true }
        Node {
            width: percent(100), height: percent(100),
            padding: UiRect { top: px(40), bottom: px(12), left: px(12), right: px(12) },
            flex_direction: FlexDirection::Column, row_gap: px(8),
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            (
                pane_header()
                Node { flex_shrink: 0.0 }
                Children [
                    (Text("Feathers / Editor widgets") ThemedText),
                    (
                        @FeathersButton { @caption: bsn! { Text("Close") ThemedText } }
                        Name("Close widget gallery")
                        AutoFocus
                        on(close_gallery)
                    ),
                ]
            ),
            (
                Node { flex_grow: 1.0, min_height: px(0), column_gap: px(8) }
                Children [
                    gallery_pane("Buttons & selection", bsn_list![super::widget_gallery::controls_column()]),
                    gallery_pane("Fields & containers", bsn_list![super::widget_gallery::properties_column()]),
                    gallery_pane("Color", bsn_list![super::widget_gallery::color_column()]),
                ]
            ),
        ]
    }
}

fn sync_gallery(
    mut commands: Commands,
    cli: Res<ClientInfo>,
    roots: Query<Entity, With<GalleryRoot>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
    mut focus: ResMut<InputFocus>,
    mut session: ResMut<GallerySession>,
) {
    let active = cli.curr_ui == CurrentUI::WidgetGallery;
    let desired_scale = if active {
        window
            .single()
            .map(|w| (w.width() / 1000.0).min(w.height() / 540.0).clamp(0.5, 1.0))
            .unwrap_or(1.0)
    } else {
        1.0
    };
    if scale.0 != desired_scale {
        scale.0 = desired_scale;
    }
    if active && roots.is_empty() {
        // 立即标记待加载的根节点，避免字体/图标加载期间每帧重复排队。
        commands.queue_spawn_scene(gallery_scene()).insert(GalleryRoot);
    } else if !active {
        for root in &roots {
            commands.entity(root).despawn();
            focus.clear();
        }
        session.previous = None;
    }
}
