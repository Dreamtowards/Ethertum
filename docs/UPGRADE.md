# 升级与稳定性改进记录

目标顺序：完成并提交 Bevy 0.19 升级；完成并提交 Bevy 0.20 升级；修复 `REVIEW.md` 中的问题；改进主菜单、实际世界列表和相关 UI。此记录不代表整个目标已经完成。

## Bevy 0.19：完成阶段验证

- 基线提交：`a0a9875`，Bevy `0.19.1`。
- 阶段提交：`3c5493b`，完成下述修复和验证；未推送。
- 玩家使用服务器网络 ID 到本地 ECS 实体的显式映射；真实实体由 `Commands::spawn` 分配。登录、位置、生成、删除与退出清理使用同一映射。
- 客户端玩家实体保存实际用户名 `Name`，供 ECS 调试和双客户端实体生命周期验证使用。
- 空区块网格保留在主世界，不进入 GPU slab 上传；隐藏空表面并清除旧碰撞体。非空表面重新启用上传和可见性。
- 草木、水面 shader 区分 `PREPASS_PIPELINE`：预通道使用 Bevy 0.19.1 的 globals binding 1，普通通道导入 `mesh_view_bindings::globals`（binding 11）。首次 GPU 验证发现统一使用 binding 11 会导致预通道布局校验失败，已据实际预通道源码修正并通过 GPU 重跑。
- 启动时创建的主相机保留到程序退出，以支持退出世界后的菜单和再次入场。
- 退出世界先发送 netcode 断线消息，再释放客户端传输资源，使内置服务器及时清理在线玩家并允许同一用户名重入。
- 修复 mobile 入口的客户端插件路径和全屏模式参数；移除过时的 Android 全局 MSAA 资源设置，相机已有 `Msaa::Off`。这里只核对了 Windows 宿主工作区编译，Android/iOS 仍未验证。

验证状态：

- Windows、仓库指定的 nightly；锁文件和依赖版本未变。
- 使用已编译的同版本 ECS/网格库，直接包含修改源文件执行四个针对性测试：全部通过。覆盖未生成实体 ID、同批延迟命令、重复位置/删除、删除后重建、清空映射后重新入场、空网格反复变化。
- 使用锁定的 Naga Oil/Naga 与本地 Bevy 0.19.1 shader 模块组合校验：terrain、foliage、liquid 全部通过。这不等同于 GPU 绑定布局或视觉验证。
- 新增的实际相机退出生命周期测试已在工作区测试中通过，覆盖两次退出后的相机保留。
- 新增 `cargo run --example world_smoke --locked`：使用真实内置服务器、隐藏窗口和离屏渲染，检查登录、跨区块移动、射线找到地形后的落地碰撞、退出、同名重入和清理，等待三张游戏截图实际保存到 `target/` 后才成功退出。使用 `ParamSet` 分离位置读写与物理查询；各进程在 `target/world-smoke/PID/` 使用独立设置目录，资产和截图路径指向项目绝对路径。验证模式关闭鼠标锁定并静音，从 y=32 的地形上方出生，并发送验证加载距离 `(2,3)`，覆盖地面碰撞区块。此验证程序已在 Windows / RTX 4070 Laptop / Vulkan（驱动 610.47）运行成功，退出码 0。
- 双客户端实际运行成功：主机使用 `--listen PORT`，第二进程使用 `--peer 127.0.0.1:PORT`；检查双向跨区块位置同步、主机断线后的远端实体删除、同名重入生成新映射实体，以及双方最终断线清理。两个进程退出码均为 0。
- 单客户端三张截图、双客户端主机三张截图和远端一张截图均实际保存。已检查单客户端入场/菜单/重入和双客户端重入/远端截图，确认地形、植被、水面、天空和菜单有输出；这不是完整视觉质量或键鼠交互验证。本轮未出现原始实体插入 panic、空网格 slab 报错或渲染布局错误。日志仍有原有 `ChunkDel` 占位处理的 ERROR 和 egui 多轮布局 ID 警告，留待 REVIEW / UI 阶段处理；离屏相机还有 `ShadowLodOrigin` 提示。
- 完整 `cargo check --workspace --all-targets --locked --offline` 已通过，包括新增示例和移动入口。首次检查发现 mobile 的两处遗留 API 错误，修复后完整重跑退出码为 0。
- 随后扩展的双客户端示例通过直接 `rustc --emit=metadata` 类型检查；最终版本完整重跑 `cargo check --workspace --all-targets --locked --offline --jobs 2`，退出码为 0。
- `cargo clippy --workspace --all-targets --locked --offline` 退出码为 1：现有 `src/util/mod.rs` 的 `as_mut(&T) -> &mut T` 和 trait 接口触发 `clippy::mut_from_ref`。未压制诊断，按原目标留到 REVIEW 的所有权/快照重构阶段修复。
- 全仓格式检查存在原有差异；只格式化本次新增文件，不进行全仓格式化。
- `cargo test --workspace --locked --offline` 退出码为 0，六个测试全部通过。

## Bevy 0.20：进行中

官方 [0.20 发布说明](https://bevy.org/news/bevy-0-20/) 和 [迁移指南](https://bevy.org/learn/migration-guides/0-19-to-0-20/) 已核对。

- 统一桌面、独立服务器和移动 workspace manifest 的 Bevy 版本；检查依赖图，避免运行时混用两个 Bevy 版本。
- `bevy_egui 0.43.1` 支持 Bevy 0.20；`egui-notify 0.23.0` 对应 egui 0.36。`bevy_common_assets` 上游 0.18 和 Leafwing 主分支已迁移到 Bevy 0.20；Avian、Renet、Inspector 有待合并的迁移分支；OBJ 当前仍需移植。采用前核对发布状态、源码和依赖图；若使用 Git 则固定提交，不把未发布支持描述为稳定支持。
- 活跃材质 shader 从 naga_oil 方言迁移到 WESL，并更新模块路径、常量及 Rust `ShaderRef`；实验 DDGI 分开记录。
- 核对 Sprite 材质、几何形状路径、事件/观察器、渲染提取和 UI API；完成对应构建、行为与 GPU 验证后单独提交。

已接入的兼容源码候选（完整构建与运行仍待验证）：

| 插件 | 上游依据 | 固定提交 |
| --- | --- | --- |
| Avian | [0.20 迁移 PR](https://github.com/avianphysics/avian/pull/1070)，`andriyDev/avian` | `34c9b008d2f40638a953d9218db48d54f661365d` |
| Renet | [0.20 迁移 PR](https://github.com/lucaspoffo/renet/pull/206)，`simgine/renet` | `495716ff33fa8e7b8999658d570a72ea348027e8` |
| Inspector | [0.20 迁移 PR](https://github.com/jakobhellermann/bevy-inspector-egui/pull/324)，`taboky-dev/bevy-inspector-egui`，egui 0.36 / bevy_egui 0.43 | `9a5b0866ec764ab9baec0f1361f4edac2ea0e764` |
| Leafwing | 主分支 manifest 使用 Bevy 0.20.0 | `afae710d2766aad32635954406a852b8c56a8071` |
| 公共资源 | 上游 0.18.0 manifest 使用 Bevy 0.20 | `e90d64399e477e233e57d5735f7daa28e84520ff` |

前置探测：在忽略的 `target/upgrade-020-probe/` 独立 manifest 中解析上述候选并执行 `cargo fetch`，成功锁定并下载依赖；探测锁文件只有一个 Bevy 版本 `0.20.0`。Avian 的 `bevy_heavy`、`bevy_transform_interpolation`、`glam_matrix_extras` 迁移依赖也使用固定 rev。探测只证明版本解析成功，不代表 Rust API、shader 或运行行为已经兼容。

0.19 提交后已切换主包和 mobile manifest，接入表中固定 rev，采用 `crates/bevy_obj` 的本地移植（基于上游 0.19.0，保留 MIT/Apache 许可证和 mesh/scene 功能）。普通 Cargo 解析增量更新工作区锁文件，未执行全仓 `cargo update`；补充下载后 `cargo fetch --locked` 成功。首次检查发现原锁文件中的 `anstream 0.6.13` 与新 `anstyle-wincon` 的 trait 实现冲突，定向更新为 `anstream 0.6.21`，其他非相关直接依赖未主动升级。

三个活跃 shader 已迁移为 `.wesl`，同步修改材质引用、模块路径、`constants::MATERIAL_BIND_GROUP` 和顶点解压；预通道 globals 与普通通道继续使用各自布局。使用与 Bevy 相同的 `wesl 0.6` / `naga-ext`、跨包解析规则及实际 0.20 模块，对 terrain/foliage/liquid 组合后执行 Naga 30 校验，三者均通过。尚未执行 0.20 GPU 验证。实验 DDGI 尚未迁移或验证。

0.20 验证进度：

- `cargo check --workspace --all-targets --locked --offline --jobs 2` 退出码 0；覆盖本地 OBJ 移植和 mobile 宿主入口。已修复 egui 0.36 的面板根 Ui、菜单栏、选择按钮、圆角和组合框 API，以及渲染适配器封装和资源加载 builder API。
- `cargo tree --workspace --locked --offline` 的活跃依赖只有 Bevy `0.20.0`。未启用的旧移动触控可选依赖仍保留在 manifest 中，不能据此宣称移动端支持已完成。
- `cargo clippy --workspace --all-targets --locked --offline --jobs 2` 仍因既有的两处 `clippy::mut_from_ref` 失败；未压制，待 REVIEW 所有权重构修复。
- 完整工作区测试正在执行。测试使用 `target/verification-profile.toml` 的命令局部配置：dev debug=0、依赖 opt-level=1，限制 jobs=2，以减小完整代码生成的内存、符号文件和等待时间；仓库默认构建配置未修改。命令为 `cargo test --workspace --locked --offline --jobs 2 --config target/verification-profile.toml`。同样的可复现配置已保存为 `.cargo/verification.toml`，后续运行和干净检出可使用 `--config .cargo/verification.toml`；这些验证不用于性能结论。
- OBJ 移植新增三角形几何/UV 翻转和无效 UTF-8 回归测试；测试运行结果仍待确认。

已下载的 0.20 源码确认：引擎 shader 使用 `.wesl`；`prepass::io` 的顶点解压函数与压缩字段需要随自定义顶点 shader 一起核对，材质 API 还涉及 `bevy::material` 路径。体素材质把 ID/光照放在 UV 中，迁移时需保留这些数据的数值精度。

## REVIEW 重构：待升级完成

逐项以当前代码验证并修复：共享区块引用强转与任务快照；输入解码及聊天命令；加载预算与数值范围；登录阻塞、token 日志及状态；独立服务器配置；服务器权威地形和同步；网络实体生命周期；面/边/角重建与碰撞同步；带版本的实际存档往返。每批修复补充失败场景测试并按需提交。

## UI 改进：待稳定性基础完成

主菜单、真实世界列表、创建/选择/加载/返回菜单流程应连接实际世界元数据与存档，取消示例世界行。评估 Bevy 0.20 原生 UI/BSN，保留游戏 HUD 和开发调试的必要能力；验证鼠标、键盘、焦点、窗口缩放及重入世界。未验证的平台和实验功能继续明确标记。

已核对 Bevy 0.20 本地官方 `examples/scene/bsn.rs`：可用 `bsn!` / `bsn_list!` 的 Rust 模板组合 `Node`、`ui_widgets::Button`、`Text` 和 `On<PointerPress>`。后续菜单可采用此方式组织可复用布局；实际世界数据、保存状态和键盘焦点仍由明确的 ECS 资源/系统管理，模板本身不代表存档已经实现。
