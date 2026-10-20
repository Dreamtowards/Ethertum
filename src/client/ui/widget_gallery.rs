//! Feathers 控件试用面板，改编自 Bevy 0.19.1 的 feathers_gallery.rs（MIT / Apache-2.0）。
//! https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/widgets/feathers_gallery.rs
//! 状态只用于演示控件，不修改游戏设置、世界数据或网络状态。
use super::CurrentUI;
use crate::client::prelude::ClientInfo;
use bevy::{
    ecs::VariantDefaults,
    feathers::{
        constants::{fonts, icons},
        containers::*,
        controls::*,
        display::{icon, label, label_small},
        font_styles::InheritableFont,
        palette,
        rounded_corners::RoundedCorners,
        theme::ThemedText,
        tokens,
    },
    input_focus::InputFocus,
    prelude::*,
    text::{EditableText, TextEdit, TextEditChange},
    ui::{Checked, InteractionDisabled, Selected},
    ui_widgets::{
        checkbox_self_update, listbox_update_selection, radio_self_update, slider_self_update, Activate, ActivateOnPress, RadioGroup,
        SliderPrecision, SliderStep, SliderValue, ValueChange,
    },
};

pub(super) fn configure(app: &mut App) {
    app.insert_resource(DemoWidgetStates {
        rgb_color: Srgba::new(0.35, 0.55, 0.8, 0.85),
        hsl_color: Hsla::new(210.0, 0.6, 0.5, 1.0),
        scalar_prop: 7.0,
        vec3_prop: Vec3::new(10.1, 7.124, 100.0),
        clicks: 0,
        last_key: String::new(),
    })
    .add_systems(
        Update,
        update_colors.run_if(|cli: Res<ClientInfo>| cli.curr_ui == CurrentUI::WidgetGallery),
    );
}

/// A struct to hold the state of various widgets shown in the demo.
#[derive(Resource)]
struct DemoWidgetStates {
    rgb_color: Srgba,
    hsl_color: Hsla,
    scalar_prop: f32,
    vec3_prop: Vec3,
    clicks: usize,
    last_key: String,
}

#[derive(Component, Clone, Copy, Default)]
struct GalleryFeedback;

fn count_click(_event: On<Activate>, mut states: ResMut<DemoWidgetStates>) {
    states.clicks += 1;
}

#[derive(Component, Clone, Copy, PartialEq, FromTemplate)]
enum SwatchType {
    #[default]
    Rgb,
    Hsl,
}

#[derive(Component, Clone, Copy, Default)]
struct HexColorInput;

#[derive(Component, Clone, Copy, Default)]
struct DemoDisabledButton;

#[derive(Component, Clone, Copy, Default)]
struct DemoScalarField;

#[derive(Component, Clone, Copy, Default, VariantDefaults)]
enum DemoVec3Field {
    #[default]
    X,
    Y,
    Z,
}

pub(super) fn controls_column() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(100),
            flex_shrink: 0.0,
        }
        Children [
            (Text("Clicks: 0") ThemedText GalleryFeedback),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (
                        @FeathersButton {
                            @caption: bsn! { Text("Normal") ThemedText }
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Normal")
                        Name("Gallery normal")
                        on(count_click)

                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { Text("Disabled") ThemedText },
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Disabled") InteractionDisabled
                        Name("Gallery disabled")
                        InteractionDisabled
                        DemoDisabledButton
                        on(count_click)
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { Text("Primary") ThemedText },
                            @variant: ButtonVariant::Primary,
                        }
                        AccessibleLabel("Primary")
                        Node {
                            flex_grow: 1.0,
                        }
                        on(count_click)
                    ),
                    (
                        @FeathersMenu
                        Children [
                            (
                                @FeathersMenuButton {
                                    @caption: bsn! { Text("Menu") ThemedText }
                                }
                                AccessibleLabel("Menu Example")
                                Node {
                                    flex_grow: 1.0,
                                }
                            ),
                            (
                                @FeathersMenuPopup
                                Children [
                                    (
                                        @FeathersMenuItem {
                                            @caption: bsn! { Text("MenuItem 1") ThemedText }
                                        }
                                        on(count_click)
                                    ),
                                    (
                                        @FeathersMenuItem {
                                            @caption: bsn! { Text("MenuItem 2") ThemedText }
                                        }
                                        on(count_click)
                                    ),
                                    @FeathersMenuDivider,
                                    (
                                        @FeathersMenuItem {
                                            @caption: bsn! { Text("MenuItem 3") ThemedText }
                                        }
                                        on(count_click)
                                    )
                                ]
                            )
                        ]
                    )
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(1),
                }
                Children [
                    (
                        @FeathersButton {
                            @caption: bsn! { Text("Left") ThemedText },
                            @corners: RoundedCorners::Left,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Left")
                        on(count_click)
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { Text("Center") ThemedText },
                            @corners: RoundedCorners::None,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Center")
                        on(count_click)
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { Text("Right") ThemedText },
                            @variant: ButtonVariant::Primary,
                            @corners: RoundedCorners::Right,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Right")
                        on(count_click)
                    ),
                ]
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { Text("Disable button") ThemedText }
                }
                Checked
                Name("Gallery checkbox")
                AccessibleLabel("Checkbox Example")
                on(
                    |change: On<ValueChange<bool>>,
                        query: Query<Entity, With<DemoDisabledButton>>,
                        mut commands: Commands| {
                        info!("Checkbox clicked!");
                        let Ok(entity) = query.single() else { return; };
                        let mut button = commands.entity(entity);
                        if change.value {
                            button.insert(InteractionDisabled);
                        } else {
                            button.remove::<InteractionDisabled>();
                        }
                        let mut checkbox = commands.entity(change.source);
                        if change.value {
                            checkbox.insert(Checked);
                        } else {
                            checkbox.remove::<Checked>();
                        }
                    }
                )
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { Text("Fast Click Checkbox") ThemedText }
                }
                ActivateOnPress
                AccessibleLabel("Fast Click Checkbox Example")
                on(
                    |change: On<ValueChange<bool>>,
                     mut commands: Commands| {
                        info!("Checkbox clicked!");
                        let mut checkbox = commands.entity(change.source);
                        if change.value {
                            checkbox.insert(Checked);
                        } else {
                            checkbox.remove::<Checked>();
                        }
                    }
                )
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { Text("Disabled") ThemedText },
                }
                InteractionDisabled
                AccessibleLabel("Disabled Checkbox Example")
                on(|_change: On<ValueChange<bool>>| {
                    warn!("Disabled checkbox clicked!");
                })
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { Text("Checked+Disabled") ThemedText }
                }
                InteractionDisabled
                Checked
                AccessibleLabel("Disabled and Checked Checkbox Example")
                on(|_change: On<ValueChange<bool>>| {
                    warn!("Disabled checkbox clicked!");
                })
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: px(4),
                        }
                        RadioGroup
                        on(radio_self_update)
                        Children [
                            (
                                @FeathersRadio {
                                    @caption: bsn! { Text("One") ThemedText }
                                }
                                Checked
                            ),
                            @FeathersRadio {
                                @caption: bsn! { Text("Two") ThemedText }
                            },
                            (
                                @FeathersRadio {
                                    @caption: bsn! { Text("Fast Click") ThemedText }
                                }
                                ActivateOnPress
                            ),
                            (
                                @FeathersRadio {
                                    @caption: bsn! { Text("Disabled") ThemedText }
                                }
                                InteractionDisabled
                            ),
                        ]
                    )
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (@FeathersToggleSwitch on(checkbox_self_update)),
                    (@FeathersToggleSwitch ActivateOnPress on(checkbox_self_update)),
                    (@FeathersToggleSwitch InteractionDisabled on(checkbox_self_update)),
                    (@FeathersToggleSwitch InteractionDisabled Checked on(checkbox_self_update)),
                    (@FeathersDisclosureToggle on(checkbox_self_update)),
                ]
            ),
            (
                @FeathersSlider {
                    @max: 100.0,
                    @value: 20.0,
                }
                SliderStep(10.)
                Name("Gallery slider")
                SliderPrecision(2)
                on(slider_self_update)
            ),
        ]
    }
}
pub(super) fn color_column() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8), flex_shrink: 0.0 }
        Children [
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(4),
                }
                Children [
                    label("Srgba"),
                    // Spacer
                    flex_spacer(),
                    // Text input
                    (
                        @FeathersTextInputContainer
                        Node {
                            flex_grow: 0.
                            padding: { px(4).left() },
                        }
                        Children [
                            (
                                @FeathersTextInput {
                                    @visible_width: 10f32,
                                    @max_characters: 9usize,
                                }
                                InheritableFont {
                                    font: fonts::MONO
                                }
                                HexColorInput
                                on(handle_hex_color_change)
                            )
                        ]
                    )
                    (@FeathersColorSwatch SwatchType::Rgb),
                ]
            ),
            (
                @FeathersColorPlane::RedBlue
                on(|change: On<ValueChange<Vec2>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.red = change.value.x;
                    color.rgb_color.blue = change.value.y;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Red
                }
                AccessibleLabel("Red Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.red = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Green
                }
                AccessibleLabel("Green Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.green = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Blue
                }
                AccessibleLabel("Blue Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.blue = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Alpha
                }
                AccessibleLabel("Alpha Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.alpha = change.value;
                })
            ),
            (
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                }
                Children [
                    label("Hsl"),
                    (@FeathersColorSwatch SwatchType::Hsl)
                ]
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslHue
                }
                AccessibleLabel("Hue Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.hue = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslSaturation
                }
                AccessibleLabel("Saturation Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.saturation = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslLightness
                }
                AccessibleLabel("Lightness Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.lightness = change.value;
                })
            )
        ]
    }
}

pub(super) fn properties_column() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(100),
            flex_shrink: 0.0,
        }
        Children [
            label_small("Text input"),
            (
                @FeathersTextInputContainer
                Children [
                    (
                        @FeathersTextInput { @max_characters: 128usize }
                        Name("Gallery text")
                        EditableText::new("Edit me")
                    ),
                ]
            ),
            (
                pane() Children [
                    pane_header() Children [
                        @FeathersToolButton {
                            @variant: ButtonVariant::Primary,
                        } on(count_click) Children [
                            (Text("\u{0398}") ThemedText)
                        ],
                        pane_header_divider(),
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } on(count_click) Children [
                            (Text("\u{00BC}") ThemedText)
                        ],
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } on(count_click) Children [
                            (Text("\u{00BD}") ThemedText)
                        ],
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } on(count_click) Children [
                            (Text("\u{00BE}") ThemedText)
                        ],
                        pane_header_divider(),
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } on(count_click) Children [
                            icon(icons::CHEVRON_DOWN)
                        ],
                        flex_spacer(),
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                        } on(count_click) Children [
                            icon(icons::X)
                        ],
                    ],
                    (
                        pane_body() Children [

                            subpane() Children [
                                subpane_header() Children [
                                    (Text("Left") ThemedText),
                                    (Text("Center") ThemedText),
                                    (Text("Right") ThemedText)
                                ],
                                subpane_body() Children [

                                    group()
                                    Children [
                                        group_header() Children [
                                            (Text("Group") ThemedText),
                                        ],
                                        group_body()
                                        Children [

                                            label_small("Scalar property"),
                                            (
                                                @FeathersNumberInput
                                                Name("Gallery scalar")
                                                DemoScalarField
                                                Node {
                                                    flex_grow: 1.0,
                                                    max_width: px(100),
                                                }
                                                on(
                                                    |value_change: On<ValueChange<f32>>,
                                                    mut states: ResMut<DemoWidgetStates>| {
                                                    if value_change.is_final {
                                                        states.scalar_prop = value_change.value;
                                                    }
                                                })
                                            ),
                                            label_small("Scalar property (copy)"),
                                            (
                                                @FeathersNumberInput
                                                DemoScalarField
                                                Node {
                                                    flex_grow: 1.0,
                                                    max_width: px(100),
                                                }
                                                on(
                                                    |value_change: On<ValueChange<f32>>,
                                                    mut states: ResMut<DemoWidgetStates>| {
                                                    if value_change.is_final {
                                                        states.scalar_prop = value_change.value;
                                                    }
                                                })
                                            ),
                                            label_small("Vec3 property"),
                                            Node {
                                                display: Display::Flex,
                                                flex_direction: FlexDirection::Row,
                                                column_gap: px(6),
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::SpaceBetween,
                                            }
                                            Children [
                                                (
                                                    @FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_X_AXIS,
                                                        @label_text: "X",
                                                    }
                                                    DemoVec3Field::X
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    BorderColor::all(palette::X_AXIS)
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        if value_change.is_final {
                                                            states.vec3_prop.x = value_change.value;
                                                        }
                                                    })
                                                ),
                                                (
                                                    @FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_Y_AXIS,
                                                        @label_text: "Y",
                                                    }
                                                    DemoVec3Field::Y
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        if value_change.is_final {
                                                            states.vec3_prop.y = value_change.value;
                                                        }
                                                    })
                                                ),
                                                (
                                                    @FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_Z_AXIS,
                                                        @label_text: "Z",
                                                    }
                                                    DemoVec3Field::Z
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        if value_change.is_final {
                                                            states.vec3_prop.z = value_change.value;
                                                        }
                                                    })
                                                ),
                                            ],
                                        ],
                                    ]
                                ],
                            ]
                        ]
                    ),
                ]
            ),
            subpane() Children [
                subpane_header() Children [
                    (Text("List") ThemedText),
                ],
                subpane_body() Children [
                    @FeathersListView {
                        @rows: {bsn_list![
                            @FeathersListRow Children [(Text("First World") ThemedText)],
                            @FeathersListRow Selected Children [(Text("Second Nature") ThemedText)],
                            @FeathersListRow Children [(Text("Third Degree") ThemedText)],
                            @FeathersListRow InteractionDisabled Children [(Text("Fourth Wall") ThemedText)],
                            @FeathersListRow Children [(Text("Fifth Column") ThemedText)],
                            @FeathersListRow Children [(Text("Sixth Sense") ThemedText)],
                            @FeathersListRow Children [(Text("Seventh Heaven") ThemedText)],
                            @FeathersListRow Children [(Text("Eighth Wonder") ThemedText)],
                            @FeathersListRow Children [(Text("Ninth Inning") ThemedText)],
                            @FeathersListRow Children [(Text("Tenth Amendment") ThemedText)],
                            @FeathersListRow Children [(Text("Eleventh Hour") ThemedText)],
                            @FeathersListRow Children [(Text("Twelfth Night") ThemedText)],
                        ]}
                    }
                    Node {
                        max_height: px(130)
                    }
                    Name("Gallery list")
                    on(listbox_update_selection)
                ],
            ],
            label_small("Virtual keyboard"),
            (
                @VirtualKeyboard::<String> {
                    @keys: vec![
                        vec!["1".into(), "2".into(), "3".into()],
                        vec!["4".into(), "5".into(), "6".into()],
                        vec!["7".into(), "8".into(), "9".into()],
                    ]
                }
                on(|event: On<VirtualKeyPressed<String>>, mut states: ResMut<DemoWidgetStates>| {
                    states.last_key = event.key.clone();
                })
            ),
        ]
    }
}

fn update_colors(
    states: Res<DemoWidgetStates>,
    mut sliders: Query<(Entity, &ColorSlider, &mut SliderBaseColor)>,
    mut swatches: Query<(&mut ColorSwatchValue, &SwatchType), With<FeathersColorSwatch>>,
    mut color_planes: Query<&mut ColorPlaneValue, With<FeathersColorPlane>>,
    mut q_text_input: Query<(Entity, &mut EditableText), With<HexColorInput>>,
    mut feedback: Query<&mut Text, With<GalleryFeedback>>,
    new_inputs: Query<Entity, Added<HexColorInput>>,
    q_scalar_input: Query<Entity, With<DemoScalarField>>,
    q_vec3_input: Query<(Entity, &DemoVec3Field)>,
    mut commands: Commands,
    focus: Res<InputFocus>,
) {
    // 返回主菜单时控件是新实体，即使资源值未变化也要重新同步。
    if states.is_changed() || !new_inputs.is_empty() {
        for mut text in &mut feedback {
            text.0 = if states.last_key.is_empty() {
                format!("Clicks: {}", states.clicks)
            } else {
                format!("Clicks: {}   Key: {}", states.clicks, states.last_key)
            };
        }
        for (slider_ent, slider, mut base) in sliders.iter_mut() {
            match slider.channel {
                ColorChannel::Red => {
                    base.0 = states.rgb_color.into();
                    commands.entity(slider_ent).insert(SliderValue(states.rgb_color.red));
                }
                ColorChannel::Green => {
                    base.0 = states.rgb_color.into();
                    commands.entity(slider_ent).insert(SliderValue(states.rgb_color.green));
                }
                ColorChannel::Blue => {
                    base.0 = states.rgb_color.into();
                    commands.entity(slider_ent).insert(SliderValue(states.rgb_color.blue));
                }
                ColorChannel::HslHue => {
                    base.0 = states.hsl_color.into();
                    commands.entity(slider_ent).insert(SliderValue(states.hsl_color.hue));
                }
                ColorChannel::HslSaturation => {
                    base.0 = states.hsl_color.into();
                    commands.entity(slider_ent).insert(SliderValue(states.hsl_color.saturation));
                }
                ColorChannel::HslLightness => {
                    base.0 = states.hsl_color.into();
                    commands.entity(slider_ent).insert(SliderValue(states.hsl_color.lightness));
                }
                ColorChannel::Alpha => {
                    base.0 = states.rgb_color.into();
                    commands.entity(slider_ent).insert(SliderValue(states.rgb_color.alpha));
                }
            }
        }

        for (mut swatch_value, swatch_type) in swatches.iter_mut() {
            swatch_value.0 = match swatch_type {
                SwatchType::Rgb => states.rgb_color.into(),
                SwatchType::Hsl => states.hsl_color.into(),
            };
        }

        for mut plane_value in color_planes.iter_mut() {
            plane_value.0.x = states.rgb_color.red;
            plane_value.0.y = states.rgb_color.blue;
            plane_value.0.z = states.rgb_color.green;
        }

        // Only update the hex input field when it's not focused, otherwise it interferes
        // with typing.
        if let Ok((input_ent, mut editable_text)) = q_text_input.single_mut() {
            if Some(input_ent) != focus.get() {
                editable_text.queue_edit(TextEdit::SelectAll);
                editable_text.queue_edit(TextEdit::Insert(states.rgb_color.to_hex().into()));
            }
        }

        for scalar_input_ent in q_scalar_input.iter() {
            commands.trigger(UpdateNumberInput {
                entity: scalar_input_ent,
                value: NumberInputValue::F32(states.scalar_prop),
            });
        }

        for (vec3_input_ent, axis) in q_vec3_input.iter() {
            let new_value = match axis {
                DemoVec3Field::X => states.vec3_prop.x,
                DemoVec3Field::Y => states.vec3_prop.y,
                DemoVec3Field::Z => states.vec3_prop.z,
            };

            commands.trigger(UpdateNumberInput {
                entity: vec3_input_ent,
                value: NumberInputValue::F32(new_value),
            });
        }
    }
}

fn handle_hex_color_change(
    _change: On<TextEditChange>,
    q_text_input: Single<&EditableText, With<HexColorInput>>,
    mut colors: ResMut<DemoWidgetStates>,
) {
    let editable_text = *q_text_input;
    if let Ok(color) = Srgba::hex(editable_text.value().to_string()) {
        if color != colors.rgb_color {
            colors.rgb_color = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct ObservedValue(f32);

    fn scalar(app: &mut App) -> Entity {
        app.world_mut()
            .spawn(DemoScalarField)
            .observe(|event: On<UpdateNumberInput>, mut commands: Commands| {
                if let NumberInputValue::F32(value) = event.value {
                    commands.entity(event.entity).insert(ObservedValue(value));
                }
            })
            .id()
    }

    #[test]
    fn returning_to_gallery_restores_values_without_changing_game_state() {
        let mut app = App::new();
        app.init_resource::<ClientInfo>().init_resource::<InputFocus>();
        configure(&mut app);
        app.world_mut().resource_mut::<ClientInfo>().curr_ui = CurrentUI::WidgetGallery;
        let old_hex = app.world_mut().spawn((HexColorInput, EditableText::new(""))).id();
        let old_scalar = scalar(&mut app);
        app.update();
        app.world_mut().resource_mut::<DemoWidgetStates>().scalar_prop = 42.0;
        app.update();
        assert_eq!(app.world().get::<ObservedValue>(old_scalar).unwrap().0, 42.0);
        app.world_mut().despawn(old_hex);
        app.world_mut().despawn(old_scalar);
        app.world_mut().resource_mut::<ClientInfo>().curr_ui = CurrentUI::Settings;
        app.update();
        app.world_mut().spawn((HexColorInput, EditableText::new("")));
        let new_scalar = scalar(&mut app);
        app.world_mut().resource_mut::<ClientInfo>().curr_ui = CurrentUI::WidgetGallery;
        app.update();
        assert_eq!(app.world().get::<ObservedValue>(new_scalar).unwrap().0, 42.0);
        assert_eq!(app.world().resource::<ClientInfo>().curr_ui, CurrentUI::WidgetGallery);
    }
}
