# Pixel Character Forge 0.2

Rust 程序化像素角色工作台：角色配方、五官与衣服、骨骼 IK、头发披风物理、动作编辑和图集导出使用同一套核心。无需预先绘制角色动画帧。

**本次升级：更高细节分辨率、减少内部黑边、可编辑动作与 20 个预设。构建与验收状态见 `VERIFICATION.md`。测试代码存在不等于已通过测试。**

## 启动

在本目录执行，首次构建需要 Rust stable 和依赖下载网络：

```sh
cargo test --no-default-features
cargo check --all-targets --all-features
cargo run --release --bin forge
```

作为游戏仓库中的独立工具时，本目录位于 `tools/pixel-character-forge`。保留独立 `[workspace]`，不修改父项目。GUI 使用 Macroquad + egui，业务核心、CLI 和像素渲染均为 Rust。

## 画面变化

默认画布从 128×128 改为 **256×256**；支持 128、192、256、384、512。这里是重新栅格化程序化形状，不是把旧的低像素图片放大。画布大小不是人物占用范围，人物周围仍保留动作与武器空间。

新增眼睑、虹膜、瞳孔、眼睛高光、鼻唇与脸颊层次、发束高光，以及衣领、纽扣、口袋、腰带扣、护甲面板、铆钉和靴口。细节仍随身体与装备参数生成，没有引入逐角色图片素材。

底部 `Outline` 有三档：`None` 完全关闭描边；默认 `SoftSilhouette` 只对最终剪影最外侧一个输出像素施加轻微、随材质颜色的暗化；`LegacyInk` 保留原型的逐部件黑线作为对照。默认强度 0.18，阴影对比度降低。头发、披风采用连续带状轮廓，不再给每段物理链单独画黑边。`Rig debug` 默认关闭。

`Fine detail` 开关控制细节层；右侧 `High-detail face inset` 由同一角色以 512 画布重新绘制并裁出头部，不使用独立头像。192 适合保留更粗的像素颗粒；256 是默认折中。384/512 便于检查五官和装备。高分辨率与脸部小窗的实际帧率仍需实机测量。

## 动作编辑

右侧 **ANIMATION STUDIO** 是独立动作资产编辑器，左侧仍为捏人和装备。

1. 在 `Preset` 选择动作，点击 `Load`。这会替换当前动作，`Animation Undo` 可恢复。先保存重要修改。
2. 暂停，拖动底部时间轴。打开 `Drag pose handles`，直接拖动根节点、胸部、头部、手脚控制点；也可以在 `Control` 中选择目标，输入 X/Y/Z。
3. 数值修改和控制点拖拽会自动写关键帧；`Set key [I]` 显式插入。旋转轨道使用角度，位移轨道使用随身高缩放的局部单位。双手武器请编辑 `RightHand` 和 `WeaponRotation`，副手保持共享握持约束。
4. 在时间轴拖动菱形移动关键帧；支持选中、删除、复制粘贴和复制到播放头。碰撞到已有关键帧时拒绝移动，不静默覆盖。
5. `Interpolation` 选择 Step、Linear、Smooth、EaseIn、EaseOut。它属于关键帧之后的区间；修改下拉选择后点击 `Set key` 应用。曲线区域显示选中控制点的三轴变化，不是贝塞尔切线编辑器。
6. `Duration / retime` 按比例调整关键帧和事件；支持循环、单次播放、镜像、帧吸附、逐帧前后移动和暂停洋葱皮。一次鼠标拖动是一项动画撤销记录。
7. `Save animation` 保存独立 JSON；换另一体型后可加载同一动作。`Save JSON` 保存的是人物，不包含动画。

快捷键：Space 播放/暂停，左右方向键逐帧，I 插入关键帧，Delete 删除选中关键帧。文本输入时不抢这些快捷键。Escape 关闭工作台。

九条控制轨道为 Root、Chest、Head、LeftHand、RightHand、LeftFoot、RightFoot、WeaponRotation、BodyRotation。它们是叠加在程序化基础动作之上的偏移，保留 IK 与共享武器框架，不是逐骨骼 FK 曲线编辑。

## 20 个动作预设

`idle` 待机、`walk` 行走、`run` 跑步、`slash` 斩击、`combo` 三段连击、`thrust` 突刺、`guard` 防御、`roll` 翻滚、`hit` 受击、`jump` 跳跃、`crouch` 蹲伏、`kick` 踢击、`uppercut` 上勾拳、`cast` 施法、`wave` 挥手、`bow` 鞠躬、`cheer` 欢呼、`sit` 坐姿、`dance` 舞蹈、`dash` 冲刺。

这些是可继续调整的程序化预设，不是已由动画师逐动作验收的商业动作包。事件标记会保存/导出，不执行游戏伤害。部分手势配长武器可能显得不合适，可改成空手或编辑动作。

## 无窗口工具

```sh
# 六个人物、透明 PNG 和排列预览；输出默认 256
cargo run --release --no-default-features --bin forge-cli -- examples exports/presets
# 将全部 20 个可编辑动作保存为 JSON
cargo run --no-default-features --bin forge-cli -- motion-presets exports/animations
# 查看动作名或检查动作资产
cargo run --no-default-features --bin forge-cli -- motions
cargo run --no-default-features --bin forge-cli -- validate-clip examples/animations/wave.json
# 将自定义动作导出为八方向图集
cargo run --release --no-default-features --bin forge-cli -- export-clip examples/clockwork-mage.json examples/animations/wave.json exports/wave 24 256
# 生成真实渲染图，不需要窗口或 GPU
cargo run --release --no-default-features --bin forge-cli -- render examples/clockwork-mage.json examples/animations/idle.json face.png 0.4 384 20 soft
```

`forge-cli --help` 列出全部命令。原有 `preset`、`validate`、`export` 命令保留。CLI 写入会覆盖同名文件；GUI 图集导出会创建时间戳目录。

## 文件兼容与导出格式

旧人物 JSON 仍使用 schema 1，可直接加载。角色、动画和渲染样式分别存储为 `character.json`、`animation.json`、`render.json`。

**图集格式升级为 schema 2，消费端需要适配。** 高分辨率可能拆成多张图片，不再保证一方向占一整行。读取 `images[frame.page].file` 和 `frame.rect` 来定位图像。首张为 `atlas.png`，后续为 `atlas-001.png` 等；每页不超过 4096×4096。透明 RGBA 像素、15 个关节、武器挂点、攻击阶段和出框标记仍导出。关节坐标相对于单格，不是整张图集。

循环动画不重复导出末帧，单次动画包含终点。总像素预算限制仍存在。长武器和极端体型可能出框，`touches_edge` 标记此情况；没有自动缩放、自动扩画布或完美遮挡承诺。

## 保留的能力与边界

六类角色起点、身体和五官调整、头发与衣装、武器、配色、程序化挂件、种子生成、分组锁定、六候选与人物撤销保留。更高细节仍由规则生成，不增加每个角色的逐帧美术工作。

动作是程序化目标 + 控制点关键帧 + IK；物理主要用于头发、披风和冲击响应。尚无完整格斗判定、全身主动布偶、接触驱动起身、布料自碰撞、复杂摔投、手指/表情轨道、动画片段混合或任意生物拓扑。

渲染仍按部件平均深度排序，并非逐像素三维深度。抬臂、翻转、极端体型与装备组合需要视觉回归。更高清不等于已达到商业美术质量。

## 工程布局

`recipe.rs` 人物配方；`math.rs` 向量/IK；`animation.rs` 20 种基础动作和物理；`clip.rs` 动作资产与关键帧；`editor.rs` 动作编辑 UI；`render.rs` 像素绘制；`lib.rs` 资产读写与分页导出；`bin/forge.rs` 桌面工作台；`bin/forge-cli.rs` 无窗口工具。

`.github/workflows/verify.yml` 适合本目录作为独立仓库；嵌入父仓库时，把 `integration/pixel-character-forge.yml` 放到父仓库 `.github/workflows/`。工作流包含核心测试、桌面编译检查、Clippy、真实渲染导出和 Linux 构建，不需要仓库写权限。
