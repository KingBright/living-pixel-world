# Living Pixel World · 0.3 源码候选

Rust 核心的俯视角像素世界 RPG。地形、水、天气、生态、生产、贸易、NPC 与玩家操作共用同一份权威世界状态。

**交付状态：源码已写入，但没有在本次环境执行 Rust 编译、测试、图形启动或跨平台验收。** 本版不是已通过验收的游戏成品，也不代表原设计全部最终功能已完成。详见 [源码覆盖与边界](docs/SOURCE_STATUS.zh-CN.md)。原始需求保留在 `docs/REQUIREMENTS.zh-CN.md` 和 `docs/DESIGN.md`。

## 启动原生客户端

在仓库根目录使用 Rust 1.85.1 工具链执行：

```sh
cargo run --release --manifest-path apps/world_client/Cargo.toml -- --new --seed 42 --size 8000 --save saves/valley
```

后续启动省略 `--new`，自动加载默认存档。`--new` 表示明确创建新世界；不要把它与需要保留的存档路径搭配使用。使用新路径可以保留旧世界。`--load saves/name` 加载指定快照，但写入目标由 `--save` 决定，两个参数应一起配置。

```sh
cargo run --release --manifest-path apps/world_client/Cargo.toml -- --load saves/valley --save saves/valley
```

客户端是 Rust 程序，窗口由 minifb 承载，逻辑 framebuffer 为 960×540，像素画面由 `world_view` 在 Rust 中生成。`--scale2` 使用二倍窗口。默认不启动音频设备；可选程序化环境音：

```sh
cargo run --release --manifest-path apps/world_client/Cargo.toml --features audio -- --save saves/valley
```

窗口依赖 minifb 0.28.0；可选音频依赖 rodio 0.20.1。Linux 图形版本需要可用的 X11 或 XWayland 环境及相关开发库，音频版本还需要 ALSA 开发库。Linux、macOS、Windows 构建工作流已编写但未运行，不能据此声称三平台可用。

**客户端独立 Cargo workspace 是为了隔离窗口和音频依赖，不是引入另一套模拟。** 根 workspace 的九个包只依赖本地 Rust crate，含本地依赖锁文件。客户端的第三方传递依赖尚未解析，`apps/world_client/Cargo.lock` 尚未生成；首次构建成功后应保留并提交该文件，再启用客户端 `--locked` 构建。

## 操作

| 键位 | 功能 |
|---|---|
| WASD / Space / Shift | 移动 / 近战 / 闪避 |
| E / G | 附近交互 / 采集菜单 |
| I / C / T / J / M / H | 背包 / 制作 / 市场 / 契约 / 地图 / 帮助 |
| 上下 / Enter / Tab | 选择 / 执行 / 买卖或存取切换 |
| B、1 至 7、左键 | 建造模式、结构类型、放置 |
| 右键 / R | 射箭 / 休息 |
| P / N / 加减键 | 暂停 / 单步 / 速度 |
| F5 / F9 / F6 / F12 | 保存 / 加载 / 导出回放 / 导出当前 BMP |

背包中 Enter 可装备有耐久的物品或消耗食物。采矿需要相应工具，捕鱼需要装备网；制作必须在真实工坊入口附近，手工配方除外。任务需要到发行聚落接受及交付。死亡后 Enter 恢复，掉落背包留在原地。界面目前使用程序内置拉丁像素字形，未实现中文 UI 排版。

存档写入 `saves/valley.slot0` 与 `.slot1`，两份应一起保留。快照保存完整状态，而不是从世界创建时重放历史。截图与回放默认写入 `captures/`。工具和窗口关闭时是否保存见具体命令，不要假定 CLI 查询自动持久化。

## 无窗口入口与 Agent 协议

以下命令是后续运行入口，不是本次已经执行的结果。

```sh
cargo run --release --locked --offline -p world_cli -- new --seed 42 --size 8000
cargo run --release --locked --offline -p world_cli -- shell --seed 42 --size 1024 --radius 0
cargo run --release --locked --offline -p world_cli -- run examples/smoke.lpw --size 1024 --radius 0
cargo run --release --locked --offline -p world_cli -- replay out/smoke.lpwr
```

CLI 的 `shell` 从标准输入逐行读指令，标准输出每行一个 JSON 响应。真实游戏操作和原生输入都转成同一种 `Action`。`status` 返回简明 JSON；`market`、`people`、`inspect` 等查询按需展开，避免每一步倾倒整个世界。协议、输入限制和例子见 `docs/PROTOCOL.md`。

```text
status
market 0
buy 0 log 1
craft split_firewood 1
wait 16
verify
save saves/agent
record captures/agent.lpwr
quit
```

`--sandbox` 必须在创建世界时明确指定，才允许人工降雨、注水、地形编辑和传送。加载世界保留存档中的权限，不会因为 CLI 额外传入 `--sandbox` 而提升权限。

## 源码布局

```text
crates/sim_core       基础时间、随机与校验
crates/world_model    细粒度区块与地表状态
crates/worldgen       原有地形生成与排水算法
crates/hydrology      细粒度守恒水量更新
crates/simulation    保留的 M1a 专项模拟测试容器
crates/world_headless 保留的 M1a 专项诊断入口
crates/world_runtime  权威游戏状态、所有系统、命令、存档与回放
crates/world_view     只读软件像素渲染、界面、程序化环境音
crates/world_cli      Agent 标准输入协议、场景和长期测试入口
apps/world_client    原生窗口、键鼠与可选音频设备
assets/catalog       实际加载的 TSV 内容定义
examples             可执行验收场景，不是结果录像
```

M1a 的 `simulation` 是专项回归夹具，不与客户端同时运行。客户端唯一权威状态是 `world_runtime::Game`。全部自研可执行代码为 Rust；TSV、Markdown、YAML 和 Cargo 清单不构成第二游戏语言。

## 内容基线

默认世界参数为 8000×8000 米，1 米地表 tile，64×64 tile 区块，活跃半径 0 至 2 区块。程序包含 80 种商品、54 个有输入输出的配方、24 种植物、13 种动物定义、150 名持久化 NPC、五个聚落及十个故事锚点。额外配方补齐油、种子、家具等投入产出链；第十三种动物是产蜜蜂群。数据数量并不等价于生态平衡或完整商品玩法已验收。

当前宏观 atlas 为全图驻留；32.768 km 是源码限制，不是原设计约 65 km 目标。32.768 km 的性能也未验证。近远场转换守住水总量和动物身份等选定不变量，但不保证与全图细粒度模拟逐步等价。

## 后续验证入口

```sh
cargo fmt --all
cargo test --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline
cargo run --release --locked --offline -p world_cli -- content
cargo run --release --locked --offline -p world_cli -- run examples/hydrology.lpw --size 1024 --radius 0 --sandbox
cargo run --release --locked --offline -p world_cli -- soak --days 1 --size 1024 --radius 0 --save out/soak
```

20 年对应本游戏历法的 2400 天：`soak --days 2400`。该入口逐 tick 执行，不用跳过模拟来伪造稳定性结果，也未做过本次长测。正式长测必须先验证小样本耗时、性能预算、存档预算、人口与价格走势，再决定实际运行参数。

`docs/VALIDATION.md` 区分已执行的文件/数据检查和未执行的 Rust 验证。`verification/source_checks_v0_3.json` 为本次静态检查记录。没有提交或推送到 GitHub。
