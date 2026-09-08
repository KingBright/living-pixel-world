# 从 v0.1 升级到 v0.2

先备份自己的角色 JSON、导出文件和本地修改。解压新版到独立目录，或将完整项目放到仓库 `tools/pixel-character-forge`；不要只复制新界面文件，动作、渲染和导出接口同时变化。

旧角色 JSON 可直接通过左侧 Load JSON 加载；动画通过右侧 Load animation 单独加载。

默认画布 256×256。只希望小幅增加像素密度时，底部 Pixels 选 192。Outline 默认 SoftSilhouette，仍嫌边缘明显可选 None。Fine detail 打开，Rig debug 关闭。

右侧 Preset 选 Wave 或 Combo，Load 后暂停；打开 Drag pose handles，移动播放头后拖手脚。时间轴拖动菱形调整节奏，右侧 Animation Undo 撤销。Save animation 保存动作资产，左侧 Export 导出当前自定义动作，不只是原预设。

图集 schema 从 1 升为 2。消费端必须通过 images[frame.page].file 与 frame.rect 读取，不能再硬编码一行一个方向。新导出同时包含人物、动作和样式 JSON。

构建：

```sh
cargo test --no-default-features
cargo check --all-targets --all-features
cargo run --release --bin forge
```

本包不包含已签名的安装程序。验证状态以 VERIFICATION.md 和实际 CI 结果为准。
