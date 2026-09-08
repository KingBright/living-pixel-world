# v0.2 验证记录

日期：2026-09-07。

## 已实际通过的构建与测试

代码提交：`212a07f698991484b97dd85ea4c1c37ae919abb6`。

GitHub Actions： https://github.com/KingBright/living-pixel-world/actions/runs/34104360093

环境：Ubuntu 24.04 x86_64，Rust 1.98.1。工作流已完成，结论 success。

- 核心集成测试：15/15 通过。
- 动作编辑与新渲染集成测试：23/23 通过。
- `cargo check --all-targets --all-features`：通过。
- `cargo clippy --all-targets --all-features`：执行成功，但有告警，并非零告警检查。
- 真实 CLI 导出：六个人物 PNG、20 个动作 JSON、连击八方向图集、128 黑边与 256 柔和轮廓 PNG 均成功。
- `cargo build --release --bins`：成功，生成 Linux `forge` 与 `forge-cli`。

告警包括 egui Stroke 浮点字面量的未来兼容提示，以及代码格式、参数数量等 Clippy 提示。没有把告警隐藏或声称已经清零。

## 已执行的本地运行检查

编辑容器没有 Rust/Cargo，但成功运行了以上 CI 产物。

桌面程序在 Linux Xvfb + 软件 OpenGL 中实际启动并截图。通过真实界面把右手 X 偏移拖到 5.2，保存动画 JSON，确认键值；点击撤销后保存得到 0.0，重做后保存恢复 5.2。还加载了 Wave 动作 JSON，时间轴定位至 0.7 秒，确认基础动作和预览改变。开启了控制点和洋葱皮显示。

通过编译后的 CLI 另外生成了 192、384 像素 PNG 和 36 帧连击图集。交付的对比图、GIF 与桌面截图均来自真实程序；对比图只是统一展示尺寸与增加标签，不是重新绘制角色。对比图左边是 v0.2 的 LegacyInk 模式，不冒充完整 v0.1 原版截图。

## 交付校验

CI 产物 ID：10011788848。

原始 CI ZIP SHA256：`b898b01e89cc8faaa6a984b77e80943e153df0fbe33c2a68a99c981addd17419`。

下载产物已核对 SHA256 与 ZIP CRC。上传的 Rust 源文件已逐一核对 Git blob SHA。源码 ZIP 另附本次 CI 的 Cargo.lock 与文件校验清单，便于复现依赖。

## 仍未证明的事项

这不是完整交互验收：尚未逐一验证所有关键帧控件、鼠标拖拽方式、窗口尺寸与输入法；所有体型、装备、方向和动作的交叉组合也没有穷举。没有 Windows/macOS 实机验证、性能基准或商业美术验收。Xvfb 试跑不代表真实显卡与高 DPI 设备全部兼容。

动作仍是基础姿态 + 编辑偏移 + IK + 次级物理，不是完整接触格斗、全身主动布偶或布料自碰撞。图集 schema 2 需要消费端适配。
