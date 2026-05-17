# Vibe2D 引擎已知坑点

记录引擎 API 不够直观、容易让游戏代码踩坑的地方。每条都标明：现象、根因、当前 workaround、建议的引擎侧修复。

---

## 1. `InputState::mouse_position()` 在游戏首帧就返回 `(0, 0)`

### 现象

游戏使用鼠标做网格 / 单位选择时，启动后会发现一些「应该在别处的 cursor」被吸到地图左上角 `(0, 0)`，即使用户根本没动鼠标。

具体复现：在 `tactics-demo` 中，`Game::new()` 把 `cursor` 设到首个我方单位的位置；`update()` 每帧调用 `collect_command(input, cursor)` 让鼠标覆盖 cursor；结果首帧 `input.mouse_position() == (0.0, 0.0)`，落在地图内 `(0, 0)` 格，cursor 被擦掉。同样的问题也会让 VDP `game.selectUnit` 之类设置的 cursor 在下一帧被擦回 `(0, 0)`。

### 根因

`crates/vibe_input/src/lib.rs:67-85` 的 `InputState::new()` 把 `mouse_x / mouse_y` 初始化为 `0.0`：

```rust
mouse_x: 0.0,
mouse_y: 0.0,
```

`mouse_position()` 直接返回这两个字段（lib.rs:173）。引擎没有提供任何 API 去区分以下两种情况：

- 「鼠标真实在 (0, 0)」（用户把光标推到屏幕左上）
- 「鼠标从未上报过位置」（OS / winit 还没发送 `CursorMoved` 事件）

也没有 `mouse_delta()`、`mouse_just_moved()`、`mouse_position_opt() -> Option<(f32, f32)>` 之类的辅助 API。

### 当前 workaround（游戏侧）

在游戏 struct 里自己保存上一帧的鼠标像素位置，只有像素位置实际变化时才让鼠标覆盖游戏内 cursor。`tactics-demo` 已采用此方案：

```rust
// examples/tactics-demo/src/main.rs
pub struct TacticsDemo {
    // ...
    pub last_mouse_px: Option<(f32, f32)>,
}

// examples/tactics-demo/src/input.rs
let cur_mouse = input.mouse_position();
let mouse_changed = prev_mouse_px.is_some_and(|(px, py)| (px, py) != cur_mouse);
if mouse_changed && let Some(g) = mouse_to_grid(cur_mouse.0, cur_mouse.1) {
    cursor = g;
}
```

`prev_mouse_px == None`（首帧）时不让鼠标覆盖任何东西。

### 建议的引擎侧修复

任选其一即可：

1. **新增 `mouse_position_opt() -> Option<(f32, f32)>`**
   - `InputState` 内部把 `mouse_x / mouse_y` 改成 `Option<(f32, f32)>`，初始 `None`
   - 直到 winit 第一次发 `CursorMoved` 才填值
   - 旧的 `mouse_position()` 保留向后兼容，对 `None` 返回 `(0.0, 0.0)`

2. **新增 `mouse_just_moved() -> bool`**
   - 类似 `is_mouse_button_just_pressed`，每帧 reset
   - 游戏代码用 `if input.mouse_just_moved() { snap_to_grid(...) }`

3. **新增 `mouse_delta() -> (f32, f32)`**
   - 类似 `mouse_scroll_delta()`，每帧 reset
   - 任何非零 delta 都意味着鼠标真的移动过

方案 1 信息最完整（区分了「从未上报」），方案 2 最贴合现有 API 风格，方案 3 最通用。

### 状态

- 引擎：未修复
- `examples/tactics-demo`：已用 workaround（commit 待补）
- 其他 examples：`flappy-bird`、`tetris`、`mari0` 等不依赖鼠标网格定位，未受影响

---

## 模板

新增坑点请按以下结构写：

```markdown
## N. <一句话标题>

### 现象
<复现路径、能观察到什么>

### 根因
<引擎代码的具体文件:行号、为什么这样写>

### 当前 workaround
<游戏侧能怎么绕开>

### 建议的引擎侧修复
<列出 1-3 个候选方案>

### 状态
- 引擎：<未修复 | PR #xx | 已合入 vX.Y>
- 受影响的 examples：<...>
```
