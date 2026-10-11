# 恢复开发审查

审查日期：2026-10-10。范围：当前工作树中的 Cargo 配置、客户端/服务器入口、网络收发、区块生命周期、地形编辑、存储骨架和平台构建入口。工作树已有依赖升级与 Bevy API 迁移修改；本次保留这些修改，仅更新文档。

这是静态审查及构建检查，不等同于完整渲染、联机、性能或安全测试。以下行号对应审查时的代码。

## 优先处理的问题

### P1：共享引用强转为可变引用，破坏区块内存安全

证据：`src/util/mod.rs:15–34` 的 `as_mut` / `AsMutRef`；`src/voxel/chunk.rs:58` 的 `at_voxel_mut(&self)`；`src/voxel/voxel_client.rs:246–265` 后台网格任务读取共享 `Arc<Chunk>`；同文件 `:443–469` 主线程通过同一类转换修改体素。

`#[allow(invalid_reference_casting)]` 只关闭诊断，不使这种转换合法。API 可以从共享引用制造互相别名的 `&mut`；编辑与后台网格读取还可能并发访问区块。即使没有观察到崩溃，也不能保证结果正确。

建议优先恢复明确所有权：主线程负责可变世界数据，后台任务使用不可变快照，结果带区块修订号；或采用有清晰锁范围的同步容器。不要仅为所有转换补一层 `unsafe` 或继续禁用 lint。先覆盖生成、邻接关系、编辑和网格读取，再移除转换接口。

### P1：网络解码和聊天命令可以触发服务端 panic

证据：`src/net/netproc_server.rs:86` 对客户端字节执行 `bincode::deserialize(...).unwrap()`；`:182–186` 对聊天命令使用 `shlex::split(...).unwrap()`、直接索引参数并 `parse().unwrap()`。客户端 `src/net/netproc_client.rs:75` 也直接 unwrap 服务端数据。

已连接客户端发送不符合协议的字节即可触发解码 panic；登录后发送 `/`、`/time`、`/time set`、`/time set abc` 或不闭合引号，也会走到不安全的解析路径。

建议：限制包长与解码资源，返回可处理错误；以完整参数匹配解析命令，校验有限数值与合法范围；加入上述失败输入的回归测试，确认服务器继续运行。

### P1：客户端提供的加载距离没有上限，任务数量限制挡不住遍历开销

证据：`src/net/netproc_server.rs:197–198` 直接接受 `LoadDistance`；`src/voxel/voxel_server.rs:54` 和 `:141` 调用 `iter_center_spread`；遍历实现见 `src/util/mod.rs:128`。

远端可以发送很大的加载半径。即使加载/发送数量受限，闭包内的 `return` 也只是结束本次回调，外部遍历仍继续，足以长时间占用服务器更新；极端坐标和半径还会增加整数运算溢出的风险。

建议：服务端限制半径、世界坐标与每帧预算，校验位置为有限值；让遍历支持提前停止或改用有预算的待加载队列。测试负半径、过大半径和非有限位置。

### P1：登录在 ECS 主循环中阻塞 800 ms，且仍是未认证原型

证据：`src/net/netproc_server.rs:112–119` 明文记录 `access_token` 并 `sleep(800ms)`；`src/net/mod.rs` 使用 `ServerAuthentication::Unsecure`；登录分支没有实际 token 验证，也没有同一连接重复登录的状态限制。

每个登录请求都阻塞处理网络与世界更新的系统；客户端内置服务器也在同一个 App 中运行，会影响画面响应。重复以不同用户名登录还能覆盖玩家记录并留下之前生成的实体。

建议：移除模拟等待和 token 日志，建立握手/登录/游戏状态机；真实认证放到可取消的异步任务中，完成后再入场。先把当前实现明确标为原型，再按实际部署需求设计认证。

### P2：独立服务器配置读取失效，人数上限也未使用配置

证据：`src/server/dedicated_server.rs:44` 解析的是文件名字符串 `server.settings.json`，没有读文件；`src/net/netproc_server.rs:39` 固定 `max_clients = 64`，默认 `ServerSettings` 对外报告 80。

因此配置文件中的端口、MOTD 和人数不会在启动时生效；状态接口报告的人数上限与传输层不一致。

建议：先读取 JSON 文件再解析，区分不存在、无效 JSON 与 I/O 错误；传输层使用校验后的 `num_player_limit`。配置加载已有 PreStartup、绑定在 Startup，可在保留此顺序的基础上修复。测试自定义配置和损坏配置。

### P2：客户端地形与服务器未形成统一权威来源

证据：`src/voxel/voxel_client.rs:143–189` 自行生成区块，网络 `ChunkNew` 也生成并插入区块；`:475` 地形修改发送已注释；`src/net/netproc_server.rs:228–234` 修改处理已注释；`src/net/netproc_client.rs:163–174` 收到 `ChunkDel` 仅打印日志。

客户端本地编辑不会成为服务端世界修改，也不会可靠同步给其他玩家；本地生成结果和服务端数据可能竞争同一位置。协议枚举存在不代表多人编辑已实现。

建议：区分离线预览和联网模式。联网时由服务器提供区块与确认编辑，客户端保留表现缓存；落实删除处理、编辑合法性与修订号，再做两客户端进入、编辑、移动卸载和重连验证。

### P1：get_or_spawn 没有保持网络实体 ID（已存在于 HEAD）

证据：`src/util/mod.rs:167–172` 的 `get_or_spawn(entity)` 在查不到目标时调用 `spawn_empty()`，产生一个新的任意实体 ID；`src/net/netproc_client.rs:103`、`:125`、`:137` 的生成和位置更新仍按 `EntityId::client_entity()` 查找目标。

新实体没有建立网络 ID 到本地实体的映射，下一次位置包仍查找旧的指定 ID，可能继续生成额外实体，删除也可能找不到目标而 panic。后续与 HEAD 对比确认，这段实现已经存在于最后提交中，不是本次未提交的 0.19 迁移引入的问题。

建议：使用显式的网络 ID → 本地 Entity 映射，把生成、移动、删除和断线清理全部接入；网络 ID 不要只依赖 ECS 索引，需考虑实体复用。验证一次 EntityNew 后多次 EntityPos 不增加实体数量，EntityDel 能删除同一实体。

### P2：区块边界编辑未完整触发相邻网格重建

证据：本地编辑在 `src/voxel/voxel_client.rs:468` 只标记被编辑区块；网络更新在 `src/net/netproc_client.rs:180–189` 使用只返回一个方向的 `Chunk::at_boundary_naive`。

边界体素是相邻区块网格的输入；修改面、边、角时遗漏邻接重建，可能留下旧表面、裂缝或旧碰撞体。网络处理中的注释也指出边角遗漏。

建议：从网格实际采样范围推导受影响区块，统一本地与网络更新的失效逻辑；覆盖面、边、角和负坐标案例，核对渲染与碰撞。

### P2：目前不存在可用的世界持久化闭环

证据：`src/voxel/chunk_storage.rs` 的 load/save 为空且未接入 `src/voxel/mod.rs`；`src/client/ui/serverlist.rs:265` 固定绘制 28 个示例世界；`src/voxel/worldgen.rs:10` 使用硬编码 seed。

设置保存不等于世界保存。退出、卸载和重新生成可能丢失地形修改；界面输入的世界信息也不能视作已经落盘。

建议：先实现单世界元数据（seed、生成器版本、存档版本）和 dirty 区块持久化；保存完成后才能清除 dirty 标记，验证重启恢复与写入失败处理，再接入世界列表。

## 恢复开发顺序

1. 固定一个可复现的 Rust / Bevy / 插件版本组合，恢复桌面客户端与独立服务器的编译；先完成当前迁移，避免同时开展另一轮大版本升级。
2. 处理共享区块安全、输入解析、加载预算和主循环阻塞，补充失败场景回归测试。
3. 跑通单人闭环：进入世界 → 编辑 → 保存 → 退出 → 重启恢复；统一边界重建与碰撞更新。
4. 完成服务器权威区块、登录状态和两客户端编辑同步，再补断线实体清理与重连。
5. 在桌面稳定后逐一验证 Web、Android、iOS 和实验 DDGI，按平台记录构建和运行结果。

## 验证记录

- 环境：Windows PowerShell；`rustc 1.100.0-nightly (e7769602a 2026-08-24)`，`cargo 1.100.0-nightly (e8cb624d5 2026-08-22)`。
- `cargo metadata --no-deps --locked --offline --format-version 1` 成功解析 workspace（`ethertia` 与 `mobile`）。
- `cargo check --workspace --all-targets --locked --offline` 开始检查 Bevy 渲染依赖，最后输出为 `Checking bevy_egui v0.40.1`；之后长时间没有新增输出，本次主动中止（退出码 1）。没有获得完整项目编译结果，不能据此判定构建通过或源码编译失败。
- `git diff --check -- README.md AGENTS.md docs/REVIEW.md` 未报告 diff 空白错误（新文件未被 Git 跟踪，另外人工核对内容）；Git 提示 README 后续可能转换为 CRLF。
- 未运行图形客户端、独立服务器、联机对照、移动/Web 构建或 DDGI；未做性能测量。
- 本次仅修改 README 并新增协作指南和此审查记录，没有修复上述运行时代码问题。
