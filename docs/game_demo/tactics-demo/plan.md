# 类火焰纹章战棋 Demo 实现计划

本文档描述一个基于 Vibe2D 当前能力实现的最小化战棋 demo。目标是对标《索菲亚的复苏》第一关的教学节奏和玩法密度，而不是复刻其素材、角色、地图或具体数值。

## 目标

在 `examples/tactics-demo` 中实现一个可运行、可测试、可通过 VDP 自动化驱动的回合制网格战棋关卡。Demo 应覆盖完整的一局体验：我方阶段选择单位、移动、攻击或待机，敌方阶段自动行动，最终胜利或失败。

## 非目标

- 不实现完整火纹系统，例如职业转职、成长率、物品栏、支援、地形动画、剧情对话、多关卡进度。
- 不使用原作素材、角色名、地图布局或数值。
- 不改动 Vibe2D 引擎核心，除非实现过程中发现现有 API 缺陷。
- 不引入 ECS、脚本系统或额外 tilemap 引擎。

## Demo 范围

MVP 内容：

- 14x10 左右的固定网格地图。
- 地形包含平原、道路、森林、据点、墙或山体障碍。
- 我方 4 个单位，敌方 5 个普通单位，1 个敌方队长。
- 鼠标和键盘均可完成基本操作。
- 显示选中单位、移动范围、可攻击目标、战斗预览、行动菜单、回合信息和战斗日志。
- 基础战斗规则包含伤害、命中、反击、追击和死亡。
- 敌方 AI 支持能攻击则攻击，否则向最近我方单位接近。
- 胜利条件：敌方全灭。
- 失败条件：我方全灭。
- VDP 支持状态查询、状态修改、直接执行游戏动作和截图测试。

## 项目文件

新增文件：

```text
examples/tactics-demo/
├── Cargo.toml
├── game.yaml
├── src/
│   ├── main.rs          # 入口 + Game trait impl + 模块 glue
│   ├── model.rs         # Unit/Map/Tile/Phase/PendingAction 等核心数据类型
│   ├── map.rs           # 固定关卡数据、地形查询、移动范围算法
│   ├── combat.rs        # 战斗预览、伤害、反击、追击、死亡结算
│   ├── ai.rs            # 敌方阶段同步 AI
│   ├── input.rs         # 鼠标/键盘输入到游戏命令的转换
│   └── vdp.rs           # inspect JSON 和自定义 VDP 方法分发
├── assets/
│   └── fonts/
│       └── ui.ttf
└── tests/
    └── vdp_tactics.rs
```

修改文件：

```text
Cargo.toml
```

在 workspace `members` 中加入：

```toml
"examples/tactics-demo",
```

## Cargo 配置

`examples/tactics-demo/Cargo.toml`：

```toml
[package]
name = "tactics-demo"
version.workspace = true
edition.workspace = true
license.workspace = true

[features]
default = ["vdp"]
vdp = ["vibe2d/vdp", "dep:serde_json"]

[dependencies]
vibe2d = { workspace = true }
serde_json = { workspace = true, optional = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen.workspace = true
wasm-bindgen-futures.workspace = true

[dev-dependencies]
vibe_test = { workspace = true, features = ["vdp"] }
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time"] }
anyhow.workspace = true
serde_json.workspace = true
```

说明：

- 不引入 `vibe_aoi`。本 demo 是离散网格战棋，移动范围和攻击范围用整数网格算法更直接。
- VDP feature 名称保持项目约定，仍叫 `vdp`。
- 模块拆分不是引擎硬性要求，现有 example 也有单文件实现；这里主动拆分是为了让玩法逻辑、VDP 和渲染边界清晰，减少后续迭代成本。

## game.yaml 计划

建议虚拟分辨率使用 `960x640`，方便同时展示地图和右侧状态面板。

```yaml
meta:
  name: "Tactics Demo"
  version: "0.1.0"

window:
  width: 960
  height: 640
  title: "Tactics Demo - Vibe2D"
  vsync: true

virtual_resolution:
  width: 960
  height: 640

assets:
  fonts:
    ui: "assets/fonts/ui.ttf:16"
    title: "assets/fonts/ui.ttf:24"
    small: "assets/fonts/ui.ttf:12"

input:
  actions:
    confirm:
      keys: ["Enter", "Space"]
      mouse_buttons: ["Left"]
    cancel:
      keys: ["Escape"]
      mouse_buttons: ["Right"]
    cursor_up:
      keys: ["Up", "W"]
    cursor_down:
      keys: ["Down", "S"]
    cursor_left:
      keys: ["Left", "A"]
    cursor_right:
      keys: ["Right", "D"]
    end_turn:
      keys: ["E"]

debug:
  vdp:
    enabled: true
    port: 9233
```

端口 `9233` 避免与现有 examples 的 `9229`、`9230`、`9232` 冲突。

## 主要数据模型

```rust
type UnitId = u32;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Title,
    PlayerSelect,
    PlayerMove,
    PlayerAction,
    PlayerAttackTarget,
    EnemyTurn,
    Victory,
    Defeat,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Faction {
    Player,
    Enemy,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TileKind {
    Plain,
    Road,
    Forest,
    Fort,
    Wall,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GridPos {
    x: i32,
    y: i32,
}

struct Tile {
    kind: TileKind,
    move_cost: u32,
    defense_bonus: i32,
    avoid_bonus: i32,
    blocks: bool,
}

struct Map {
    width: i32,
    height: i32,
    tiles: Vec<Tile>,
}

impl Map {
    fn in_bounds(&self, pos: GridPos) -> bool;
    fn tile(&self, pos: GridPos) -> Option<&Tile>;
    fn is_blocked(&self, pos: GridPos) -> bool;
}

struct Weapon {
    name: &'static str,
    might: i32,
    hit: i32,
    min_range: i32,
    max_range: i32,
}

struct Unit {
    id: UnitId,
    name: &'static str,
    class_name: &'static str,
    faction: Faction,
    pos: GridPos,
    hp: i32,
    max_hp: i32,
    strength: i32,
    skill: i32,
    speed: i32,
    defense: i32,
    move_range: u32,
    weapon: Weapon,
    acted: bool,
    alive: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuChoice {
    Attack,
    Wait,
    Cancel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingAction {
    None,
    Selected {
        unit_id: UnitId,
    },
    Moved {
        unit_id: UnitId,
        from: GridPos,
        to: GridPos,
    },
    ChoosingAttack {
        unit_id: UnitId,
        from: GridPos,
        target: Option<UnitId>,
    },
}

struct TacticsDemo {
    phase: Phase,
    turn: u32,
    map: Map,
    units: Vec<Unit>,
    selected: Option<UnitId>,
    cursor: GridPos,
    reachable: Vec<GridPos>,
    attackable: Vec<GridPos>,
    pending_action: PendingAction,
    combat_log: Vec<String>,
    ai_enabled: bool,
    white_tex: TextureId,
    unit_disc_tex: TextureId,
    unit_ring_tex: TextureId,
}
```

`PendingAction` 的职责是保存“当前流程的可撤销上下文”。尤其是 `Moved { from, to }`：玩家移动后进入行动菜单时，若选择 Cancel，需要把单位位置恢复到 `from`，重新计算移动范围，并回到 `PlayerMove`。VDP 的测试直控方法可以绕过这条 UI 流程，但鼠标/键盘交互必须维护该状态。

## 地图与布局

推荐布局：

- 地图区：左侧 `14 * 40 = 560` 宽，`10 * 40 = 400` 高。
- HUD 区：右侧从 `x = 600` 开始。
- 战斗日志：底部或右下方，最多显示最近 6 条。
- 行动菜单：选中单位移动后，在右侧显示 `Attack / Wait / Cancel`。

渲染实现：

- `Renderer::create_white_pixel_texture()` 注册 1x1 白纹理，用 `draw_sprite_tinted()` 绘制地块、面板和高亮。
- `Renderer::create_filled_circle_texture()` 注册单位圆形纹理。
- `Renderer::create_ring_texture()` 注册选中单位或危险范围轮廓。
- 文本统一通过 `update_ui()` 的 Label/Button/Panel 渲染，字体使用 `game.yaml` 中声明的字体。`Screen::draw_text()` 当前只支持白色文本，HUD、战斗预览、日志和按钮都不要在 `draw()` 中直接绘制文本。

颜色建议：

- 平原：草绿色。
- 道路：灰褐色。
- 森林：深绿色。
- 据点：蓝灰色。
- 墙或山：深灰色。
- 可移动范围：半透明蓝。
- 可攻击范围：半透明红。
- 我方单位：蓝色。
- 敌方单位：红色。
- 已行动我方单位：降低亮度。

## 输入交互

鼠标：

1. 鼠标移动更新 hover grid。
2. 左键点击我方未行动单位：选中并显示移动范围。
3. 左键点击可移动格：移动到目标格，进入行动菜单。
4. 左键点击敌方目标：执行攻击。
5. 右键取消当前选择或回退一步。

键盘：

1. 方向键或 WASD 移动 cursor。
2. Enter 或 Space 确认。
3. Escape 取消。
4. E 结束玩家阶段。

## 回合状态机

初始：

```text
Title -> PlayerSelect
```

玩家阶段：

```text
PlayerSelect
  select unit -> PlayerMove

PlayerMove
  choose reachable tile -> record PendingAction::Moved { from, to } -> PlayerAction
  cancel -> PlayerSelect

PlayerAction
  attack available -> PlayerAttackTarget
  wait -> PlayerSelect
  cancel -> restore PendingAction::Moved.from -> PlayerMove

PlayerAttackTarget
  choose target -> resolve combat -> PlayerSelect
  cancel -> PlayerAction
```

阶段切换：

```text
all player units acted or end_turn -> EnemyTurn
EnemyTurn resolves all enemies -> PlayerSelect
all enemies dead -> Victory
all players dead -> Defeat
```

取消语义：

- `PlayerMove` 取消：清空 `selected`、`reachable`、`attackable` 和 `pending_action`，回到 `PlayerSelect`。
- `PlayerAction` 取消：仅允许从 `PendingAction::Moved` 回退；恢复单位到 `from`，重新计算该单位从原位出发的 `reachable`，回到 `PlayerMove`。
- `PlayerAttackTarget` 取消：不改变单位位置，回到 `PlayerAction`。

## 移动范围算法

使用 Dijkstra 或小规模 BFS：

- 起点是单位当前位置。
- 每个地形提供 `move_cost`。
- `blocks = true` 的格子不可进入。
- 敌我单位占据格不可穿越，起点除外。
- 总消耗小于等于单位 `move_range` 的格子加入 `reachable`。

由于地图很小，直接用 `VecDeque` 或二叉堆均可。为了实现简单，可用 `Vec<GridPos>` 扫描最小 cost。

## 攻击范围算法

两种范围：

- 当前所在位置的直接攻击范围。
- 移动后可攻击目标范围。

MVP 中，在单位已选择时：

1. 计算所有 reachable 格。
2. 对每个 reachable 格枚举曼哈顿距离在武器范围内的格。
3. 若格上存在敌方单位，加入 `attackable`。

进入 `PlayerAttackTarget` 时，只展示从当前单位实际位置可攻击的敌人。

## 战斗规则

战斗预览：

```rust
damage = max(1, attacker.strength + weapon.might - defender.defense - tile.defense_bonus)
hit = clamp(weapon.hit + attacker.skill * 2 - defender.speed * 2 - tile.avoid_bonus, 0, 100)
double_attack = attacker.speed - defender.speed >= 4
counter = defender.alive && defender.weapon can reach attacker
```

距离判定使用攻击发生时的当前位置。也就是说，若玩家先移动再攻击，反击距离用移动后的攻击者坐标与防守者坐标计算，不使用本回合移动前坐标。

MVP 建议第一版默认命中，避免测试不稳定。若要加入命中随机：

- 使用固定 seed。
- 在 `game.inspect` 暴露 seed。
- VDP 提供 `game.setRngSeed`。

战斗结算顺序：

1. 攻击者攻击。
2. 防守者若存活且射程允许，反击。
3. 攻击者若速度差达到追击条件且仍存活，再攻击一次。
4. 任意单位 HP <= 0 时标记 `alive = false`。
5. 攻击者 `acted = true`。
6. 更新胜负状态。

## 敌方 AI

MVP 中 `EnemyTurn` 采用单帧同步结算：进入敌方阶段的那一帧，按顺序执行所有敌人行动，然后立即完成阶段收尾并回到 `PlayerSelect`。这样实现最简单，VDP 测试也只需要 `engine.step {"frames": 1}` 或等待 `frame_count` 前进一帧即可断言结果。后续如果要做逐个敌人的移动动画，再把 `EnemyTurn` 拆成跨帧子状态。

每个敌人依次行动：

1. 若当前格可攻击任一我方单位，选择 HP 最低的目标攻击。
2. 否则计算可移动范围。
3. 选择一个移动后到最近我方单位曼哈顿距离最小的格。
4. 移动后若可攻击，执行攻击。
5. 标记敌人已行动。

敌方阶段结束：

- 重置我方 `acted = false`。
- 重置敌方 `acted = false`。
- `turn += 1`。
- 回到 `PlayerSelect`。

## UI 计划

使用 `update_ui()` 构建：

- 顶部：`Turn N - Player Phase / Enemy Phase`
- 右侧面板：
  - 选中单位名称、职业、HP、攻击、防御、速度、武器。
  - 地形信息。
  - 战斗预览。
- 行动菜单：
  - Attack
  - Wait
  - Cancel
- 底部日志：
  - 最近 6 条战斗或阶段信息。

UI 控件应设置稳定 ID，方便 `ui.listWidgets` 和 VDP 测试：

- `btn_attack`
- `btn_wait`
- `btn_cancel`
- `btn_end_turn`
- `combat_log`

UI 与世界绘制边界：

- `draw()` 只画地图、范围高亮、单位图标、单位 HP 条等不需要文字颜色控制的世界元素。
- `update_ui()` 画全部文本、按钮、面板和日志；这也让 `ui.listWidgets` 能观察行动菜单与关键 HUD。
- 需要在 UI 显示动态文本时，若未来引入非 ASCII 文本，应参考 `examples/ui` 在 `update()` 中调用 `ctx.prepare_text(...)`。

## VDP 接口

VDP 方法分两类：

- 流程模拟方法：`game.selectUnit`、`game.moveSelected`、`game.waitSelected`、`game.endTurn`，遵守当前 `phase`，用于验证玩家流程。
- 测试直控方法：`game.attack`、`game.previewCombat`、`game.setUnitPos`、`game.setUnitHp`、`game.setAiEnabled`，允许绕过 UI phase，用于布置确定性测试场景。

如果流程模拟方法在错误 phase 被调用，返回 `Err("...")`，不要静默修正状态。测试直控方法仍需校验单位存在、阵营合法、目标存活和射程规则；它们只是绕过菜单流程，不绕过核心规则。

### `game.inspect`

返回完整可测试状态：

```json
{
  "phase": "player_select",
  "turn": 1,
  "selected": 1,
  "cursor": [2, 6],
  "map": {
    "width": 14,
    "height": 10,
    "tiles": [["plain"]]
  },
  "units": [
    {
      "id": 1,
      "name": "Alen",
      "class": "Fighter",
      "faction": "player",
      "x": 1,
      "y": 6,
      "hp": 22,
      "max_hp": 22,
      "acted": false,
      "alive": true
    }
  ],
  "reachable": [[2, 6], [3, 6]],
  "attackable": [[7, 6]],
  "winner": null,
  "combat_log": ["Player phase"]
}
```

### 自定义方法

| 方法 | 参数 | 说明 |
| --- | --- | --- |
| `game.reset` | `{}` | 重置整局 demo |
| `game.selectUnit` | `{ "id": 1 }` | 选中单位并计算移动范围 |
| `game.moveSelected` | `{ "x": 4, "y": 6 }` | 移动已选单位 |
| `game.waitSelected` | `{}` | 当前单位待机 |
| `game.attack` | `{ "attacker": 1, "target": 5 }` | 执行攻击 |
| `game.previewCombat` | `{ "attacker": 1, "target": 5 }` | 返回战斗预览 |
| `game.endTurn` | `{}` | 结束玩家阶段 |
| `game.setUnitPos` | `{ "id": 1, "x": 3, "y": 5 }` | 测试用传送单位 |
| `game.setUnitHp` | `{ "id": 5, "hp": 1 }` | 测试用设置 HP |
| `game.setAiEnabled` | `{ "enabled": false }` | 开关敌方 AI，便于测试 |

所有 VDP 相关代码必须用 `#[cfg(feature = "vdp")]` 门控。

## 测试计划

### 纯逻辑测试

放在对应模块的 `#[cfg(test)]` 中：

- `map.rs`：地图、地形、移动范围。
- `combat.rs`：战斗预览与结算。
- `ai.rs`：敌方行动决策。
- `model.rs`：基础状态辅助方法。

需要覆盖：

- 地形 move cost 正确。
- 不可进入墙体。
- 不能穿越单位占据格。
- 移动范围不越界。
- 攻击范围根据武器 min/max range 正确。
- 战斗 preview 与实际结算一致。
- 击杀后单位不可再被选择或作为阻挡。
- 全敌死亡进入 victory。
- 全我方死亡进入 defeat。
- 敌方 AI 能攻击时优先攻击。
- 敌方 AI 不能攻击时向最近我方接近。
- `PendingAction::Moved` 取消后能恢复移动前坐标。

### VDP 集成测试

放在 `examples/tactics-demo/tests/vdp_tactics.rs`。

测试使用 `vibe_test::GameHarness`：

```rust
const GAME_PACKAGE: &str = "tactics-demo";
const VDP_PORT: u16 = 9233;
```

测试用例：

1. `initial_state_is_valid`
   - 启动游戏。
   - `game.inspect`。
   - 断言 phase、地图尺寸、单位数量、初始 HP。

2. `select_unit_exposes_reachable_tiles`
   - `engine.pause`。
   - `game.selectUnit { id: player_id }`。
   - 断言 `reachable` 非空且不包含墙。

3. `move_selected_changes_position`
   - 选择单位。
   - 移动到一个 reachable 格。
   - `game.inspect` 断言坐标变化，phase 进入 player_action。

4. `attack_reduces_hp_or_kills`
   - 通过 `game.setUnitPos` 布置攻击者和目标。
   - `game.attack`。该方法是测试直控方法，不要求当前 phase 是 `PlayerAttackTarget`，但仍校验阵营、存活和射程。
   - 断言目标 HP 下降或死亡。

5. `end_turn_runs_enemy_phase`
   - `game.endTurn`。
   - `engine.step { "frames": 1 }`。
   - 断言回到玩家阶段，turn 增加。
   - 断言至少一个敌方单位移动或发生攻击日志。

6. `all_enemies_dead_wins`
   - 对所有敌方单位调用 `game.setUnitHp { hp: 0 }`。
   - step 一帧。
   - 断言 phase 是 victory。

7. `screenshot_writes_png`
   - 调用 `game.screenshot`。
   - step 等待。
   - 断言输出文件存在且非空。

运行：

```bash
rtk cargo test -p tactics-demo
rtk cargo test -p tactics-demo -- --ignored --test-threads=1
```

## 验收标准

功能验收：

- 可以从玩家阶段完整打一局，直到胜利或失败。
- 鼠标和键盘都能操作核心流程。
- 移动范围、攻击范围和行动状态清晰可见。
- 敌方 AI 至少能造成威胁，不需要复杂战术。
- UI 不遮挡地图核心操作区。

工程验收：

- 不改引擎核心。
- `cargo test -p tactics-demo` 通过。
- `cargo test -p tactics-demo -- --ignored --test-threads=1` 在有窗口/GPU 环境下通过。
- 所有 VDP 方法未知分支返回 `Err(format!("Unknown method: {method}"))`。
- VDP feature 关闭后不拉取 `serde_json` 相关游戏自定义代码。

## 实施里程碑

### Milestone 1: Scaffold 和静态渲染

- 新增 `examples/tactics-demo` crate。
- 建立 `model/map/combat/ai/input/vdp` 模块骨架。
- 配置 `game.yaml`、字体、workspace member。
- 注册白纹理、单位圆形纹理、单位选中 ring。
- 绘制地图、单位、基础 HUD。
- 实现 `game.inspect` 的初始状态。

完成标准：

- `rtk cargo run -p tactics-demo` 能打开窗口。
- `rtk cargo test -p tactics-demo` 编译通过。
- VDP `game.inspect` 能返回地图和单位。

### Milestone 2: 玩家选择和移动

- 实现 cursor、鼠标格子转换。
- 实现单位选择。
- 实现移动范围算法。
- 绘制移动范围。
- 实现移动到目标格和取消。
- 用 `PendingAction::Moved` 记录移动前后坐标，确保取消可恢复。

完成标准：

- 玩家能选中单位并移动。
- VDP 测试可断言 `reachable`、移动后的坐标，以及取消后恢复原坐标。

### Milestone 3: 战斗和胜负

- 实现攻击范围。
- 实现战斗预览。
- 实现攻击、反击、追击、死亡。
- 实现胜利和失败检测。
- 增加战斗日志。

完成标准：

- 玩家可击杀敌人。
- 全灭敌方后进入 victory。
- 单测覆盖核心战斗公式。

### Milestone 4: 敌方阶段

- 实现敌方 AI。
- 实现单帧同步敌方阶段与下一回合重置。
- 实现敌方攻击日志。

完成标准：

- 玩家 end turn 后敌方会移动或攻击。
- `engine.step { "frames": 1 }` 后敌方阶段结束并回到玩家阶段。

### Milestone 5: UI 和 VDP 测试完善

- 补行动菜单。
- 补稳定 UI widget ID。
- 完成 VDP 自定义方法。
- 完成 ignored VDP 集成测试。
- 补截图验收。

完成标准：

- 可以通过 VDP 自动打通关键路径。
- 截图输出正常。

## 风险与处理

- 窗口/GPU 集成测试在 headless 环境可能无法运行：保留 `#[ignore]`，默认测试只跑纯逻辑；最终在本地桌面环境运行 ignored 测试。
- 文本或 UI 面板可能溢出：右侧面板固定宽度，日志限制最近 6 条，长文本截断或缩短。
- 随机命中会导致测试不稳定：第一版默认命中，后续如加随机必须固定 seed。
- 敌方 AI 复杂度膨胀：MVP 只做贪心，不做全局战术。

## 推荐下一步

先实现 Milestone 1。该阶段能快速验证：

- example crate wiring 是否正确。
- 当前引擎绘制网格、字体、程序化纹理是否足够。
- VDP 端口和 inspect 是否可用。

Milestone 1 完成后，再进入移动与战斗逻辑，风险会低很多。
