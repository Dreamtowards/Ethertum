# 项目协作指南

## 项目定位

Ethertum 是仓库名；游戏、Cargo 主包与默认可执行文件名为 `ethertia`。
这是 Rust / Bevy 体素沙盒原型，包含客户端、内置服务器、独立服务器及移动端入口。
存档、服务端权威地形编辑和正式账号认证尚未完成，不要把占位 UI 或协议定义描述为已经实现的功能。

## 开始工作

- 先阅读 `README.md`、`Cargo.toml`、`rust-toolchain` 与相关模块；历史审查记录在 `docs/REVIEW.md`，具体问题以当前代码为准。
- 先运行 `git status --short`，保留已有的未提交修改。不要顺手回滚、全仓格式化、升级依赖或改写锁文件。
- 依赖版本以当前 manifest 和 `Cargo.lock` 为准。Bevy API、插件和 WGSL 变更需要一起核对；不要直接套用旧版示例。
- 使用中文沟通；代码标识符沿用现有英文命名。

## 模块边界

- `src/main.rs`：桌面客户端启动。
- `src/bin/dedicated_server.rs`：独立服务器启动。
- `src/client/`：角色控制、输入、世界生命周期、设置、egui UI 和实验性 DDGI。
- `src/server/`：内置/独立服务器、服务器配置与 HTTP 状态接口。
- `src/net/`：Renet 传输、协议及收发处理。
- `src/voxel/`：体素、区块、生成、网格、材质与客户端/服务端区块管理。
- `src/item/`、`src/util/`：物品注册及工具。
- `assets/shaders/`：自定义 WGSL；`assets/baked/`：运行所需图集。
- `assets/_refer_code/`：参考代码，不是编译入口；`chunk_storage.rs` 目前没有接入模块树。
- `crates/mobile/`、`build/`、`.github/workflows/`：移动端入口、打包和平台构建流程。

## 实现约束

- 区块边长是 16；`chunkpos` 是对齐到 16 的世界坐标，不是区块编号。保留负坐标语义，网络局部索引范围是 `0..4096`。
- 不要新增从 `&T` 转成 `&mut T` 的转换，或继续扩大 `AsMutRef` 的使用。现有转换是待修复的安全问题；修改共享区块时明确所有权和同步策略。
- 后台生成/网格任务尽量读取独立快照，结果经 channel 回到 ECS；设计退出、取消和过期结果处理，避免后台任务访问正在编辑的数据。
- 不在 Bevy 主循环执行 `sleep` 或阻塞网络请求。网络输入应有大小、数值范围和状态校验；解码失败、无效聊天命令不能 panic。
- 不记录 access token。登录协议目前是原型，新增功能不能假设已完成认证或权限校验。
- 改协议时同时更新客户端和服务端，评估 `PROTOCOL_ID`、bincode 枚举兼容性；持久化格式需要明确版本。
- 跨区块编辑需考虑面、边、角相邻区块的重建，以及碰撞体与网格同步。
- 图集、shader 顶点布局与材质 ID 相互关联，修改时一起核对。未经视觉验证不要宣称渲染正常。
- `ddgi` / `ddgi2`、Web 和移动端按实验/待验证路径处理，不能从存在 feature 或 CI 文件推断支持已经可用。

## 验证

从仓库根目录运行，优先保留锁文件：

```sh
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked
cargo fmt --all -- --check
```

缓存齐备时可加 `--offline`。先记录已有失败，避免为了修复无关问题扩大范围。
文档修改只需核对内容与命令；行为修复补充针对失败场景的测试，例如异常数据包、负坐标、退出后任务结果和存档往返。
不要默认用 `--all-features`：实验功能应按本次改动单独检查。
需要运行时验证时：`cargo run --locked` 或 `cargo run --bin dedicated_server --locked`。
客户端验证包括进入世界、移动/碰撞、编辑区块边界、退出并重新进入；联机改动用两个客户端核对同步与断线清理。
说明运行环境、执行过的检查和未验证的部分。除非用户明确要求，不提交、推送或发布。
