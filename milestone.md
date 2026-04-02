# Vibe2D 里程碑一：完整 Flappy Bird 游戏引擎

**日期:** 2026-04-02
**状态:** 已完成

## 概述

从零开始，用 Rust 构建了一个完整的 2D 游戏引擎，并以 Flappy Bird 作为里程碑 demo。引擎采用 Love2D/Ebiten 风格的简洁 Game trait API，支持 YAML 配置、wgpu GPU 渲染、字体渲染、音频播放，以及基于 WebSocket 的 AI 调试协议（VDP）。

## 实现步骤

### 步骤 1：项目脚手架 + 窗口 + 输入
- 创建 Cargo workspace，包含 10 个 crate
- winit 0.30 窗口创建与事件循环
- wgpu 24 surface + device 初始化
- 键盘输入状态追踪与 action 映射（YAML 配置）

### 步骤 2：精灵渲染
- wgpu sprite batch 渲染器，正交投影
- 虚拟分辨率缩放（512x288 → 1280x720，nearest filtering）
- PNG 纹理加载（`image` crate）
- AssetManager 生命周期管理：`std::mem::take` swap 模式

### 步骤 3-6：Flappy Bird 玩法实现
- 参考 Love2D 版本实现 10 层视差滚动（速度公式 150/i）
- 小鸟物理：重力 500、跳跃冲量 -200、速度上限
- 管道生成：随机间隙高度、固定间距 1.5s
- AABB 碰撞检测
- 通过管道计分

### 步骤 7：文本渲染
- 集成 fontdue 0.9 进行 TTF 字形 atlas 光栅化
- 512xN RGBA atlas 纹理，按字形 UV 查找
- Screen 提供 `draw_text()` 和 `draw_text_centered()` API

### 步骤 8：状态机
- 四状态游戏：Idle → Countdown → Playing → Dead
- 待机状态小鸟悬浮动画（正弦波）
- 3-2-1 倒计时
- Game Over 画面（分数 + 最高分 + 重试提示）

### 步骤 9：VDP（Vibe Debug Protocol）
- WebSocket + JSON-RPC 2.0 服务端（tokio-tungstenite）
- mpsc channel 通信（游戏线程 ↔ VDP 服务线程）
- 内置方法：`engine.info`、`game.inspect`、`game.screenshot`
- 游戏自定义方法：`game.setBirdY`、`game.setScore`、`game.setState`

### 步骤 10：CLI + 截图 + 音频
- `vibe` CLI 工具（clap）：new、inspect、rpc、screenshot、version 子命令
- GPU 截图捕获：离屏纹理（COPY_SRC）→ staging buffer（MAP_READ）→ BGRA→RGBA 转换 → PNG
- rodio 0.20 音频引擎：WAV 加载 + 即时播放（flap 和 hurt 音效）

---

## VDP 全流程验证记录（含自动游玩）

以下所有输出均为 2026-04-02 实际运行数据，未经任何伪造。
测试脚本：`examples/flappy-bird/tests/vdp_full_test.py`

### 游戏启动

```bash
cd examples/flappy-bird && cargo run -p flappy-bird
```

**启动日志（时间戳 2026-04-02T09:08:14Z）：**

```
INFO vibe_debug::server: VDP server starting on ws://127.0.0.1:9229
INFO vibe_debug::server: VDP server listening on ws://127.0.0.1:9229
INFO vibe_audio: Audio initialized
INFO vibe_asset: Loaded font 'ui' at 16px
INFO vibe_asset: Loaded font 'score' at 32px
INFO vibe_audio: Loaded sound 'flap'
INFO vibe_audio: Loaded sound 'hurt'
```

引擎成功初始化：VDP 服务在 `ws://127.0.0.1:9229` 监听，音频系统就绪，2 个字体和 2 个音效加载完毕。

---

### 阶段一：引擎信息查询

#### 测试 1：engine.info

```
>>> {"jsonrpc": "2.0", "id": 1, "method": "engine.info"}
<<< {
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "engine": "vibe2d",
    "version": "0.1.0",
    "virtual_height": 288.0,
    "virtual_width": 512.0
  }
}
```

**验证结果：** 引擎名称 `vibe2d`，版本 `0.1.0`，虚拟分辨率 512x288，与 game.yaml 配置一致。

---

### 阶段二：初始状态检查

#### 测试 2：game.inspect — 查询初始游戏状态

```
>>> {"jsonrpc": "2.0", "id": 2, "method": "game.inspect"}
<<< {
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "best_score": 0,
    "bird": {
      "height": 27.0,
      "vy": 0.0,
      "width": 36.0,
      "x": 128.0,
      "y": 129.41099548339844
    },
    "countdown_timer": 0.0,
    "pipes": [],
    "score": 0,
    "state": "idle"
  }
}
```

**验证结果：** 游戏处于 `idle` 待机状态，小鸟位于 (128, ~129)（屏幕中央附近），无管道，分数和最高分均为 0。y 坐标非整数是因为 idle 状态有正弦悬浮动画。

#### 测试 3-4：game.setState → idle + 验证

```
>>> {"jsonrpc": "2.0", "id": 3, "method": "game.setState", "params": {"state": "idle"}}
<<< {"jsonrpc": "2.0", "id": 3, "result": {"state": "idle"}}

>>> {"jsonrpc": "2.0", "id": 4, "method": "game.inspect"}
<<< {
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "best_score": 0,
    "bird": {"height": 27.0, "vy": 0.0, "width": 36.0, "x": 128.0, "y": 128.6141815185547},
    "countdown_timer": 0.0,
    "pipes": [],
    "score": 0,
    "state": "idle"
  }
}
```

**验证结果：** 状态确认为 `idle`，小鸟 y 从 129.41 变为 128.61（悬浮动画持续运行）。

---

### 阶段三：游戏启动（countdown → playing）

#### 测试 5-6：进入倒计时

```
>>> {"jsonrpc": "2.0", "id": 5, "method": "game.setState", "params": {"state": "countdown"}}
<<< {"jsonrpc": "2.0", "id": 5, "result": {"state": "countdown"}}

>>> {"jsonrpc": "2.0", "id": 6, "method": "game.inspect"}
<<< {
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "best_score": 0,
    "bird": {"height": 27.0, "vy": 0.0, "width": 36.0, "x": 128.0, "y": 127.87374877929688},
    "countdown_timer": 2.991830825805664,
    "pipes": [],
    "score": 0,
    "state": "countdown"
  }
}
```

**验证结果：** 状态切换为 `countdown`，倒计时器 ≈ 2.99s（从 3.0 开始），管道和分数已清零。

#### 测试 7：等待倒计时结束 → 自动进入 playing

等待 3.3 秒后查询：

```
>>> {"jsonrpc": "2.0", "id": 7, "method": "game.inspect"}
<<< {
  "jsonrpc": "2.0",
  "id": 7,
  "result": {
    "best_score": 0,
    "bird": {
      "height": 27.0,
      "vy": -50.00786209106445,
      "width": 36.0,
      "x": 128.0,
      "y": 98.48333740234375
    },
    "countdown_timer": -0.00826705526560545,
    "pipes": [],
    "score": 0,
    "state": "playing"
  }
}
```

**验证结果：** 倒计时结束后自动进入 `playing`。小鸟 y=98.48（初始跳跃后上升中），vy=-50.01（负值表示正在上升）。

---

### 阶段四：自动游玩 — VDP 操控小鸟穿越管道

测试脚本通过高频控制循环（每 30ms）实现自动游玩：
- 每 tick 调用 `game.inspect` 获取游戏状态
- 找到最近的管道，计算间隙中心位置
- 调用 `game.setBirdY(y, vy=0)` 将小鸟定位到间隙中心并重置速度

#### 测试 8：自动操控通过 2 个管道

```
开始自动操控，目标分数: 2
策略: 每 30ms 通过 setBirdY(y, vy=0) 将小鸟定位到管道间隙中心

[tick 0]   无管道，保持 bird_y=120 (实际=98.1)
[tick 30]  管道 x=550 gap_y=55 → bird_y 目标=42 (实际=42.0, vy=16.3)
[tick 60]  管道 x=300 gap_y=55 → bird_y 目标=42 (实际=42.1, vy=16.7)
[tick 89]  ★ 得分变为 1！(bird_y=42.1, vy=16.7, 管道数=2)
[tick 90]  管道 x=349 gap_y=72 → bird_y 目标=59 (实际=58.9, vy=16.6)
[tick 120] 管道 x=99  gap_y=72 → bird_y 目标=59 (实际=58.9, vy=16.7)
[tick 125] ★ 得分变为 2！(bird_y=58.9, vy=16.7, 管道数=2)
[tick 125] 已达到目标分数 2，停止操控，等待自然死亡...
```

**验证结果：**
- tick 0-29：等待第一个管道生成，保持安全高度
- tick 30-88：第一个管道 gap_y=55，将小鸟定位到 y=42（间隙中心 - 鸟高/2），成功通过
- tick 89：**得分 1** — 管道从小鸟右侧移过，计分逻辑触发
- tick 90-124：第二个管道 gap_y=72，将小鸟定位到 y=59，成功通过
- tick 125：**得分 2** — 达到目标分数，停止操控

控制精度：实际 bird_y 与目标偏差 < 1 像素（42.0 vs 42, 58.9 vs 59），证明 30ms 控制间隔 + vy 归零策略有效。

---

### 阶段五：自然死亡

#### 测试 9：停止操控后等待死亡

```
小鸟不再受控，等待重力和碰撞...
→ 小鸟已死亡！
→ 最终分数: 2
→ 最高分: 2
→ 小鸟位置: y=224.0
```

**验证结果：** 停止操控后，小鸟受重力自由下落，撞击地面（y=224.0），状态变为 `dead`，best_score 自动更新为 2。

#### 测试 10：game.inspect — 验证最终死亡状态

```
>>> {"jsonrpc": "2.0", "id": 274, "method": "game.inspect"}
<<< {
  "jsonrpc": "2.0",
  "id": 274,
  "result": {
    "best_score": 2,
    "bird": {
      "height": 27.0,
      "vy": 0.0,
      "width": 36.0,
      "x": 128.0,
      "y": 224.0
    },
    "countdown_timer": -0.00826705526560545,
    "pipes": [
      {"gap_y": 70.47019958496094, "scored": false, "x": 202.00413513183594},
      {"gap_y": 62.97431945800781, "scored": false, "x": 502.0021057128906}
    ],
    "score": 2,
    "state": "dead"
  }
}
```

**验证结果：** score=2, best_score=2，小鸟落地 y=224.0，场上仍有 2 根管道（已通过的管道被移除，新生成的未通过管道保留）。

---

### 阶段六：远程修改验证

#### 测试 11：game.setBirdY — 远程修改小鸟位置和速度

```
>>> {"jsonrpc": "2.0", "id": 275, "method": "game.setBirdY", "params": {"y": 100.0}}
<<< {
  "jsonrpc": "2.0",
  "id": 275,
  "result": {
    "bird_y": 100.0,
    "bird_vy": 0.0
  }
}
```

**验证结果：** `setBirdY` 支持同时设置位置和速度（可选 `vy` 参数），返回值同时包含 bird_y 和 bird_vy。

#### 测试 12：game.setScore — 远程修改分数

```
>>> {"jsonrpc": "2.0", "id": 276, "method": "game.setScore", "params": {"score": 99}}
<<< {
  "jsonrpc": "2.0",
  "id": 276,
  "result": {
    "score": 99
  }
}
```

#### 测试 13：game.inspect — 验证远程修改结果

```
>>> {"jsonrpc": "2.0", "id": 277, "method": "game.inspect"}
<<< {
  "jsonrpc": "2.0",
  "id": 277,
  "result": {
    "best_score": 2,
    "bird": {
      "height": 27.0,
      "vy": 8.297791481018066,
      "width": 36.0,
      "x": 128.0,
      "y": 100.10330200195312
    },
    "countdown_timer": -0.00826705526560545,
    "pipes": [
      {"gap_y": 70.47019958496094, "scored": false, "x": 202.00413513183594},
      {"gap_y": 62.97431945800781, "scored": false, "x": 502.0021057128906}
    ],
    "score": 99,
    "state": "dead"
  }
}
```

**验证结果：** bird_y ≈ 100.1（setBirdY 设置的 100.0 加上微小重力偏移），score=99（setScore 生效），best_score 仍为 2（死亡时真实得分）。

---

### 阶段七：截图

#### 测试 14：game.screenshot — VDP 远程截图

```
>>> {"jsonrpc": "2.0", "id": 278, "method": "game.screenshot", "params": {"path": "/tmp/vdp_milestone_screenshot.png"}}
<<< {
  "jsonrpc": "2.0",
  "id": 278,
  "result": {
    "path": "/tmp/vdp_milestone_screenshot.png",
    "status": "queued"
  }
}
```

截图分辨率 512x288（虚拟分辨率），PNG 格式。画面显示 Game Over 界面，Score: 99，Best: 2，管道可见，与 VDP 查询数据完全一致。

---

### 阶段八：错误处理

#### 测试 15：调用不存在的方法

```
>>> {"jsonrpc": "2.0", "id": 279, "method": "game.nonexistent", "params": {"foo": "bar"}}
<<< {
  "jsonrpc": "2.0",
  "id": 279,
  "error": {
    "code": -32000,
    "message": "Unknown method: game.nonexistent"
  }
}
```

**验证结果：** 服务端正确返回 JSON-RPC 错误响应，错误码 -32000，消息清晰说明方法不存在。

---

### CLI 工具验证

#### vibe inspect

```bash
cargo run -p vibe-cli -- inspect --addr ws://127.0.0.1:9229
```

```json
{
  "id": 1,
  "jsonrpc": "2.0",
  "result": {
    "best_score": 2,
    "bird": {"height": 27.0, "vy": 0.0, "width": 36.0, "x": 128.0, "y": 224.0},
    "countdown_timer": -0.00826705526560545,
    "pipes": [
      {"gap_y": 70.47019958496094, "scored": false, "x": 202.00413513183594},
      {"gap_y": 62.97431945800781, "scored": false, "x": 502.0021057128906}
    ],
    "score": 2,
    "state": "dead"
  }
}
```

#### vibe rpc engine.info

```bash
cargo run -p vibe-cli -- rpc engine.info --addr ws://127.0.0.1:9229
```

```json
{
  "id": 1,
  "jsonrpc": "2.0",
  "result": {
    "engine": "vibe2d",
    "version": "0.1.0",
    "virtual_height": 288.0,
    "virtual_width": 512.0
  }
}
```

**验证结果：** CLI 工具均正常工作，数据与 Python WebSocket 测试一致。

---

## 项目架构

```
vibe2d/
  Cargo.toml                    # Workspace 根配置
  crates/
    vibe2d/                     # 主引擎：Game trait, run(), Context, Screen
    vibe_render/                # wgpu sprite batch 渲染器、字体 atlas、截图
    vibe_platform/              # winit 桌面平台抽象
    vibe_input/                 # 输入状态 + action 映射
    vibe_asset/                 # 纹理/字体加载 + AssetManager
    vibe_debug/                 # VDP WebSocket 服务 + JSON-RPC 协议
    vibe_physics/               # （占位，待实现）
    vibe_audio/                 # rodio 音频引擎（WAV 加载 + 播放）
  tools/
    vibe-cli/                   # CLI 工具：vibe new/inspect/rpc/screenshot/version
  examples/
    flappy-bird/                # 完整 Flappy Bird（~480 行游戏代码）
      src/main.rs
      game.yaml
      assets/                   # 13 张纹理 + 2 个字体 + 2 个音效
  skills/
    vdp.md                      # LLM skill 文档
```

## 关键技术决策

| 决策点 | 选择 | 原因 |
|---|---|---|
| 游戏 API | Ebiten/Love2D 风格 `Game` trait (new/update/draw) | 简洁，AI 可直接生成，无 ECS 复杂性 |
| 配置格式 | YAML (game.yaml) | 声明式，人类可读 |
| 渲染 | wgpu 24，sprite batch，正交投影 | GPU 加速，跨平台 |
| 字体渲染 | fontdue 0.9，glyph atlas | 纯 Rust，无系统依赖 |
| 音频 | rodio 0.20，WAV 解码 | 轻量，跨平台，无 C 依赖 |
| 资源生命周期 | `std::mem::take` swap 模式 | 避免 Context 上的生命周期标注 |
| 调试协议 | WebSocket + JSON-RPC 2.0 (tokio-tungstenite) | 标准化，AI 可解析，类 Chrome DevTools |
| 线程通信 | std::sync::mpsc channels | 简洁，游戏循环无需 async |
| 截图实现 | 离屏纹理 (COPY_SRC) → staging buffer (MAP_READ) → PNG | Surface 纹理不支持 COPY_SRC usage |

## 下一个里程碑（Milestone 2）

- Tilemap 渲染
- Rapier2D 物理集成
- 音频增强（音量控制、循环 BGM）
- 文件变更热重载
- WASM/Web 导出目标
- 更多示例游戏
