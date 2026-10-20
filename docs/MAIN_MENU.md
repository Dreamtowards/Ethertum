# egui 主菜单与可选 Feathers 控件测试

主菜单恢复为 `src/client/ui/main_menu.rs` 中原来的 egui 界面。正常启动不会显示 Feathers 控件库，也不会生成测试页的原生 UI 实体。

## 开关与返回

在顶部 egui 菜单栏的 **View → Bevy UI / Feathers widgets** 勾选开启控件测试页。取消勾选、点击测试页的 **Close** 或按 **Esc** 都会关闭测试页并返回打开前的界面。

`CurrentUI::WidgetGallery` 是独立的测试页面，不替代 `MainMenu`。`src/client/ui/editor_widgets.rs` 保存之前的页面；如果断线、退出世界或其他 UI 操作已经切换页面，过期的关闭事件不会覆盖新页面。在世界中开启测试页时角色控制暂停接收输入，关闭后按原有页面状态恢复。

正常 egui 页面保留原来的键盘输入。仅测试页的 Tab/Enter/Space 在 egui `begin_pass` 前过滤，交给 Feathers，顶部菜单栏仍可用鼠标操作。没有把原生控件伪装成 egui 控件：egui 提供测试入口，控件页由 Bevy UI 绘制。

## 控件与状态

`src/client/ui/widget_gallery.rs` 改编自 Bevy 0.19.1 的 `examples/ui/widgets/feathers_gallery.rs`，直接使用 Feathers 默认主题与 Rust `bsn!`，包括按钮、菜单、复选框、单选组、开关、滑条、文本输入、数值输入、Vec3 字段、列表、虚拟键盘、RGBA/HSL 颜色控件，以及 pane/subpane/group 容器。

三栏各有独立滚动区域和滚动条，列表还有内部滚动区域。按钮操作显示点击计数；复选框切换示例按钮的禁用状态；两个标量字段同步同一数值；颜色平面、滑条、色块与十六进制输入同步；虚拟键盘显示最后一次输入。演示资源独立于游戏设置、世界和网络状态，不代表已经实现世界编辑器、存档、正式认证或联机地形修改。

测试页通过 `queue_spawn_scene` 等待字体与图标，并立即标记根节点以防重复排队。关闭时销毁全部子控件、清理焦点、恢复 `UiScale`；重新开启时从演示资源同步数值。直接由控件本身保存的状态（如普通滑条、列表选择和文本草稿）随控件重建。

Cargo 只启用 Bevy 0.19.1 已有的 `bevy_feathers` 功能，没有升级依赖或改写锁文件，没有新增 UI 相机。上游示例的 MIT 许可保留在 [bevy-gallery-MIT.txt](licenses/bevy-gallery-MIT.txt)。

## 验证入口

`cargo run --example editor_widgets_smoke --locked --offline` 使用真实渲染器，先验证恢复的 egui 主菜单及单人入口，再通过 egui 的 View 菜单复选框开启、关闭和重新开启控件页，验证原生 Close 按钮、控件点击、禁用状态切换、滑条拖动、缩放、滚动和实体/焦点清理。

示例从实际 egui 绘制输出取得按钮文字位置，向 egui 输入队列发送鼠标事件；原生控件使用统一 `WindowEvent` 输入。截图和运行数据隔离在 `target/editor-widgets-smoke/<pid>`，不打开外部链接。

工作区测试覆盖默认关闭、返回原界面（包括设置和世界）、重复打开、过期关闭、输入过滤、控件重建后的演示数值恢复。全仓格式检查仍有原有差异，只格式化新控件模块及示例，不全仓格式化。

2026-10-11 在 Windows / RTX 4070 Laptop / Vulkan 环境完成验证：工作区 `cargo check`、`cargo clippy` 和 29 个测试通过；上述运行示例通过，截图保存在 `target/editor-widgets-smoke/8720`。Esc 返回逻辑通过编译及页面恢复单元测试，本次运行示例没有模拟 Esc 按键，也没有重复执行进入世界或双客户端联机验证。
