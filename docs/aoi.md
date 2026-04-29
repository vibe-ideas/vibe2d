# Vibe2D AOI 模块设计

> 本文档是 `vibe_aoi` 模块的完整设计文档，记录设计动机、架构决策、API 形态、与其他模块的边界以及实施路径。在动手实现前以本文为准；实现完成后，本文档作为该模块的设计参考长期维护。

## 概述

`vibe_aoi`（Area of Interest）是 Vibe2D 的**空间查询模块**，回答「**谁在哪儿**」这一类问题：

- 鼠标点中了哪个 sprite？
- 玩家附近 200 像素内有哪些敌人？
- 哪些 sprite 在当前相机视口内（视口裁剪）？
- 实体进入/离开某个区域（触发器）？
- 一条射线打到了什么（视线、子弹）？

它**只做空间查询**，不做物理响应（速度、冲量、约束）；那是 `vibe_physics` 的职责。

### 设计目标

- **对 AI / LLM 友好**：API 概念极少（一个 `AoiWorld` + `Shape` + `EntityId`），用户读三行示例就会用
- **零侵入**：默认**不进 `Context`**，作为独立工具库被游戏代码直接持有；不用的游戏零开销
- **纯 CPU、纯 Rust**：不依赖 `wgpu` / `winit` / `tokio`，可在任何 headless 环境跑单元测试
- **VDP 友好**：可选支持把 AOI 状态暴露给 VDP，便于调试器可视化和自动化测试断言
- **2D 像素游戏的甜点区**：实体规模 < 10⁵，世界有界，实体大小相近——这是 Vibe2D 真实的目标场景

### 不做什么

- ❌ **物理响应**（速度积分、碰撞解算、约束）—— 是 `vibe_physics` 的活
- ❌ **ECS**（Vibe2D 反 ECS）—— `EntityId` 只是个 `u32` newtype，用户自己映射到游戏对象
- ❌ **Quadtree / BVH**（暂不实现）—— 等 Uniform Grid 真的不够用了再加
- ❌ **服务端 AOI / 视野同步**（暂不实现）—— API 的 `Observer` 概念为此预留了空间
- ❌ **OBB / 多边形**（暂不实现）—— 2D 游戏 90% 用 AABB + Circle 就够

---

## 与 `vibe_physics` 的关系

这两个 crate 在「碰撞检测」上有重叠，必须先把边界划清楚。

一个完整的物理流程通常分三阶段：

```
1. Broadphase（粗筛）       → 用空间分区找出"可能相撞的对"
   ↓
2. Narrowphase（细测）      → 精确判定相交、算出穿透深度/法线
   ↓
3. Resolution（响应）       → 速度反弹、冲量、约束求解
```

职责映射：

| 阶段 | `vibe_aoi` | `vibe_physics` |
|------|-----------|---------------|
| **Broadphase**（空间分区） | ✅ 核心 | ❌ 不做（直接调 aoi） |
| **Narrowphase**（粗略相交） | ✅ 是否相交、是否包含点 | ✅ 完整（穿透深度、接触点、法线） |
| **Resolution**（动力学） | ❌ | ✅ 核心 |
| **空间查询**（鼠标拾取、视口裁剪、感知范围） | ✅ 独有 | ❌ |
| **触发器事件**（enter / leave） | ✅ 独有 | ❌ |
| **运动学**（速度积分、重力） | ❌ | ✅ |
| **约束**（弹簧、关节、距离） | ❌ | ✅ |

**健康的依赖方向**：

```
游戏代码
   ├── vibe_aoi          （独立可用：纯空间查询，无需物理）
   └── vibe_physics      （依赖 vibe_aoi 做 broadphase + 几何判定）
            ↓
        vibe_aoi
```

`vibe_physics` 未来实现时，应**直接复用 `vibe_aoi` 做 broadphase + 几何判定**，自己只负责 narrowphase 接触信息和动力学积分。**绝不允许** physics 自己再写一套 grid。

### 一句话区分

> **AOI 回答"谁在哪儿"，Physics 回答"接下来怎么动"。**
>
> AOI 是 Physics 的子集（broadphase + 基础几何），但 AOI 单独存在也有价值（纯查询场景）；Physics 不应单独存在——它必须站在 AOI 的肩膀上。

### 用具体场景看清边界

| 场景 | 用谁 | 理由 |
|------|------|------|
| 鼠标点中了哪个 sprite | **aoi** | 纯空间查询 |
| 视口裁剪（哪些 sprite 在屏幕内） | **aoi** | 纯空间查询 |
| 玩家附近 200 像素内的敌人 | **aoi** | 纯空间查询 |
| 进入区域触发剧情 | **aoi**（observer） | 纯触发器，无碰撞响应 |
| 子弹打中敌人扣血 | **aoi** + 游戏代码扣血 | 不需要反弹 |
| Flappy Bird 撞管子游戏结束 | **aoi** | 撞了就结束，不需要响应 |
| Mario 踩到地面停下来 | **physics** | 需要穿透解算 + 速度归零 |
| 弹珠台小球反弹 | **physics** | 需要法线 + 反射速度 |
| 平台跳跃角色控制 | **physics** | 需要重力 + 地面响应 |

**规律**：Vibe2D 真实目标场景下 90% 的游戏只需要 **aoi**。

---

## 数据结构选型：Uniform Grid

### 为什么不是 Quadtree / BVH

| 维度 | Uniform Grid | Quadtree |
|------|--------------|----------|
| **空间结构** | 二维数组 `Vec<Vec<EntityId>>` | 递归树结构 |
| **划分依据** | 几何位置（坐标除法） | 实体密度（自适应） |
| **插入复杂度** | **O(1)** | O(log n)（可能触发分裂） |
| **查询复杂度** | O(k)，k = 查询区域覆盖的 cell 数 | O(log n + m) |
| **内存** | 预分配，跟世界大小有关 | 按需分配，跟实体分布有关 |
| **缓存友好度** | **极好**（连续数组） | 较差（指针跳转） |
| **更新代价（移动）** | **O(1)**（旧 cell 移除 + 新 cell 插入） | O(log n)（可能分裂/合并） |
| **实现复杂度** | **极简**（约 100 行） | 中等（300+ 行） |

**Vibe2D 选择 Uniform Grid 的理由**：

1. 目标场景（2D 像素小游戏）实体分布相对均匀、世界有界、实体大小相近——是 grid 的甜点区
2. 现代 CPU cache 让 grid 在中等规模（< 10 万实体）下普遍**比 quadtree 快**
3. 实现 100 行 vs 300+ 行——AI 友好度（代码可读性）差很多
4. Vibe2D 不会有「100km 开放世界 + 极不均匀分布」的场景；真有需要时更应该上 **Hash Grid** 而不是 quadtree

### Uniform Grid 与「九宫格」的关系

> **Uniform Grid 是数据结构；九宫格是它的一种典型查询模式。**

九宫格指：查询某点周围时，**只检查它所在 cell + 周围 8 个 cell，共 9 个**。

```
┌────┬────┬────┐
│ NW │ N  │ NE │
├────┼────┼────┤
│ W  │ ★  │ E  │   ← ★ 是查询点所在 cell
├────┼────┼────┤
│ SW │ S  │ SE │
└────┴────┴────┘
```

成立的前提：`cell_size ≥ 实体最大半径 + 查询半径`。这样任何可能相交的实体一定落在 3×3 cell 内。

Uniform Grid 上的查询模式不止九宫格：

| 查询类型 | 扫描的 cell 数 | 场景 |
|---------|---------------|------|
| **点查询** | 1 个 cell | 鼠标拾取 |
| **小范围圆**（半径 ≤ cell_size） | 9 个 cell（九宫格） | 玩家周围的敌人 |
| **大范围圆 / AABB** | k×k 个 cell | 视口裁剪 |
| **射线** | 沿射线穿过的 cell（DDA 算法） | 子弹、视线检测 |

`vibe_aoi` 第一阶段**不暴露专门的"九宫格"API**——`query_circle` 自动算覆盖范围即可，避免增加用户心智负担。等真的有性能需求再考虑 `query_nearby_3x3`。

### Uniform Grid 的两个短板（决定何时换方案）

1. **`cell_size` 难选**：太小则一个实体跨多 cell，重复登记；太大则一个 cell 装太多实体，退化成线性扫描。经验值：`cell_size ≈ 最常见实体直径的 1~2 倍`。如果实体大小差 100 倍，怎么选都不对——这时换 quadtree 或 BVH。
2. **世界大小决定内存下限**：100km × 100km、cell_size = 64 → 225 万 cell，即使全空也要分配。Quadtree / Hash Grid 没这个问题。

Vibe2D 不会撞上这两个短板，所以 Uniform Grid 就是终态方案。

---

## Crate 结构

### 位置

新建 `crates/vibe_aoi/`，与 `vibe_physics` 平级。

```
crates/
  vibe_aoi/
    Cargo.toml
    src/
      lib.rs        — 公共导出
      shape.rs      — Shape 枚举（Point / Circle / Aabb）+ 几何判定
      world.rs      — AoiWorld（高层 API + 后端选择）
      grid.rs       — UniformGrid 后端实现
      bruteforce.rs — BruteForce 后端实现（小规模快速通道）
      observer.rs   — Observer + enter/leave 事件
      vdp.rs        — VDP 序列化 + handle_vdp helper（feature-gated）
```

### Cargo.toml

```toml
[package]
name = "vibe_aoi"
version.workspace = true
edition.workspace = true
license.workspace = true

[features]
default = []
vdp = ["dep:serde", "dep:serde_json"]

[dependencies]
glam.workspace = true
serde = { workspace = true, optional = true, features = ["derive"] }
serde_json = { workspace = true, optional = true }
```

**严格的依赖纪律**：
- **不依赖** `wgpu` / `winit` / `tokio` / `vibe_render` / `vibe_platform` / `vibe2d`
- **只依赖** `glam`（坐标用 `Vec2`，与引擎其它部分一致）
- **可选依赖** `serde` + `serde_json`（仅 `vdp` feature 下）

这样 `vibe_aoi` 是一个**纯 CPU 工具库**，能在任何 headless 环境编译和测试。

---

## 核心 API

### 类型

```rust
/// 实体在 AoiWorld 中的句柄。用户用它把 AOI 实体映射回游戏对象。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

/// 观察者句柄，对应一个会持续追踪 enter/leave 的查询区域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObserverId(pub u32);

/// 实体或查询区域的几何形状。
#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Point(Vec2),
    Circle { center: Vec2, radius: f32 },
    Aabb { center: Vec2, half_extents: Vec2 },
}

impl Shape {
    pub fn point(p: Vec2) -> Self { Self::Point(p) }
    pub fn circle(center: Vec2, radius: f32) -> Self { Self::Circle { center, radius } }
    pub fn aabb(center: Vec2, half_extents: Vec2) -> Self { Self::Aabb { center, half_extents } }

    pub fn aabb_bounds(&self) -> (Vec2, Vec2) { /* 返回包围盒 min/max */ }
}

/// Observer 报告的事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AoiEvent {
    Enter(EntityId),
    Leave(EntityId),
}
```

### `AoiWorld`

```rust
pub struct AoiWorld { /* ... */ }

impl AoiWorld {
    /// 创建一个 uniform grid 后端的 world。cell_size 自动按 bounds 估算。
    pub fn new(bounds: Vec2) -> Self;

    /// 显式选择 brute-force 后端（适合 < 200 实体的小游戏）。
    pub fn with_bruteforce() -> Self;

    /// 显式指定 cell_size（高级用法）。
    pub fn with_grid(bounds: Vec2, cell_size: f32) -> Self;

    // ── 实体管理 ───────────────────────────────────
    pub fn insert(&mut self, shape: Shape) -> EntityId;
    pub fn update(&mut self, id: EntityId, shape: Shape);
    pub fn remove(&mut self, id: EntityId);
    pub fn get(&self, id: EntityId) -> Option<Shape>;
    pub fn len(&self) -> usize;
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, Shape)> + '_;

    // ── 查询（一次性）──────────────────────────────
    /// 返回 AABB 区域内的所有实体 id。
    pub fn query_aabb(&self, min: Vec2, max: Vec2) -> Vec<EntityId>;
    /// 返回与圆相交的所有实体 id。
    pub fn query_circle(&self, center: Vec2, radius: f32) -> Vec<EntityId>;
    /// 返回包含该点的所有实体 id（鼠标拾取用）。
    pub fn query_point(&self, p: Vec2) -> Vec<EntityId>;
    /// 射线投射，返回第一个被命中的实体 + 距离。
    pub fn raycast(&self, origin: Vec2, dir: Vec2, max_dist: f32) -> Option<(EntityId, f32)>;

    // ── Observer（持续追踪 enter/leave）─────────────
    pub fn create_observer(&mut self, region: Shape) -> ObserverId;
    pub fn update_observer(&mut self, id: ObserverId, region: Shape);
    pub fn remove_observer(&mut self, id: ObserverId);
    /// 取出该 observer 自上次调用以来累积的 enter/leave 事件。
    pub fn drain_events(&mut self, id: ObserverId) -> Vec<AoiEvent>;

    // ── 性能/调试 ─────────────────────────────────
    pub fn stats(&self) -> AoiStats;
}

pub struct AoiStats {
    pub entity_count: usize,
    pub cell_count: usize,
    pub max_entities_per_cell: usize,
    pub avg_entities_per_cell: f32,
}
```

### 用户视角的最小示例

```rust
use vibe2d::prelude::*;
use vibe_aoi::{AoiWorld, Shape, AoiEvent};

struct MyGame {
    aoi: AoiWorld,
    player_id: vibe_aoi::EntityId,
    observer: vibe_aoi::ObserverId,
}

impl Game for MyGame {
    fn new(ctx: &mut Context) -> Self {
        let mut aoi = AoiWorld::new(Vec2::new(2048.0, 2048.0));
        let player_id = aoi.insert(Shape::circle(Vec2::ZERO, 16.0));
        // 注册一个会持续追踪"玩家周围 200 像素内有谁"的 observer
        let observer = aoi.create_observer(Shape::circle(Vec2::ZERO, 200.0));
        Self { aoi, player_id, observer }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32, input: &InputState) {
        // 移动 player → 同步到 AOI
        let new_pos = /* ... */;
        self.aoi.update(self.player_id, Shape::circle(new_pos, 16.0));
        self.aoi.update_observer(self.observer, Shape::circle(new_pos, 200.0));

        // 处理感知事件
        for ev in self.aoi.drain_events(self.observer) {
            match ev {
                AoiEvent::Enter(id) => { /* 敌人进入感知范围 */ }
                AoiEvent::Leave(id) => { /* 敌人离开 */ }
            }
        }

        // 鼠标拾取（按需查询）
        let mouse = input.mouse_position();
        for hit in self.aoi.query_point(mouse) {
            // hit 是被点中的实体
        }
    }

    fn draw(&self, ctx: &Context, screen: &mut Screen) { /* ... */ }
}
```

---

## 集成方式：**不进 `Context`**

这是关键架构决策。两个备选方案：

### 方案 A：作为独立工具库，不进 `Context`（**采用 ✅**）

用户像用任何 Rust 库一样用它，自己管 `AoiWorld` 字段。

**优点**：
- 不污染 `Context`，不增加 take/swap 字段（每加一个都是引擎核心的负担）
- 用户完全控制实体 id 分配，避免和游戏自己的 entity 系统打架
- 不用的游戏（Flappy / Tetris）零开销，连依赖都不会拉
- 符合 Vibe2D「最小概念」哲学—— `Game` trait 始终只有 new / update / draw

**缺点**：
- 引擎自身（如未来的 VDP 引擎级方法）没法直接知道 aoi 状态
- 但这个能力可以由游戏通过 `handle_vdp()` 转发实现，不影响第一阶段

### 方案 B：作为 `Context.aoi` 字段（**不采用 ❌**）

需要：
- 加 `vibe2d` 对 `vibe_aoi` 的依赖
- 在 `GameBridge` 三个 take/swap 位置都加
- 用户即使不用也得带上

只有当我们要做引擎级的 AOI 调试器内省（比如「列出所有 AOI 实体并自动可视化」）时才值得这么做。第一阶段不需要。

**结论**：选方案 A。将来真要做引擎级可视化，再升级到方案 B。

---

## VDP 集成（可选，feature-gated）

VDP 集成是 Vibe2D 的特色，AOI 也应该支持。但**让游戏决定是否暴露**，不在引擎层硬塞。

### `AoiWorld::handle_vdp` helper

`vibe_aoi` 在 `vdp` feature 下提供一个 helper，游戏在自己的 `handle_vdp()` 里转发即可：

```rust
#[cfg(feature = "vdp")]
fn handle_vdp(&mut self, method: &str, params: &serde_json::Value)
    -> Result<serde_json::Value, String>
{
    if let Some(rest) = method.strip_prefix("aoi.") {
        return self.aoi.handle_vdp(rest, params);
    }
    Err(format!("Unknown method: {}", method))
}
```

### 提供的 VDP 方法（`aoi.*` 命名空间）

| 方法 | 参数 | 返回 | 用途 |
|------|------|------|------|
| `aoi.list` | — | `[{id, shape}]` | 列出所有实体（调试可视化） |
| `aoi.queryAabb` | `{min, max}` | `[id]` | AABB 查询 |
| `aoi.queryCircle` | `{center, radius}` | `[id]` | 圆形查询 |
| `aoi.queryPoint` | `{point}` | `[id]` | 点查询 |
| `aoi.raycast` | `{origin, dir, maxDist}` | `{id, distance} \| null` | 射线 |
| `aoi.stats` | — | `AoiStats` | grid 占用率（性能调优） |

这样调试器可以可视化空间分布，写自动化测试时也能直接 `await client.send("aoi.queryCircle", ...)` 验证「玩家附近真的有 3 个敌人」。

---

## 演示示例：`examples/aoi-demo`

### 设计

> **核心场景**：一张地图上随机分布若干灰色散点，3 个不同颜色的圆在地图上自动巡航，撞到边缘反弹。圆覆盖到的散点变成对应圆的颜色（演示 enter 事件），离开后变回灰色（演示 leave 事件）。鼠标 hover 时散点描白边（演示 `query_point`）。右上角实时显示 stats 面板。

### 演示了什么

| 设计点 | 体现的 AOI 能力 | 可视化 |
|--------|----------------|--------|
| 散点随机分布 | 大量实体注册到 AoiWorld | ✅ 直观看到分布 |
| 圆移动 | 高频 `update()` 调用（每帧一次） | ✅ 动画驱动 |
| 散点变色 | `query_circle` + Observer **enter** 事件 | ✅ 颜色对比强烈 |
| 离开还原 | Observer **leave** 事件 | ✅ 颜色恢复 |
| 撞墙反弹 | World bounds 检测 | ✅ 物理感（实际是 aoi 边界） |
| 多个圆 | Observer 是多对多的（一个实体能属于多个观察者） | ✅ 展示并发覆盖 |
| 鼠标 hover | `query_point` API | ✅ 描白边 |
| stats 面板 | `AoiStats` + `vibe_ui` 协作 | ✅ 数字佐证性能 |

### 推荐参数

- 窗口：600 × 400
- 散点：约 512 个，灰色，半径 4
- 圆：3 个（红 / 绿 / 蓝），半径 50，速度各异，撞墙反弹
- 重叠规则：散点同时被多个圆覆盖时显示**最近一次 enter 的圆的颜色**
- VDP 端口：9232（避开 flappy-bird=9229、ui-demo=9230、tetris=9231）

### 不加的东西

- ❌ 散点不会动（增加复杂度但不增加 AOI 演示价值）
- ❌ 不加分数 / 计分（是游戏概念，不是 AOI 概念，会模糊重点）
- ❌ 不加键盘控制圆（自动运动比手动操控更适合做演示和自动化测试）

### 配套 VDP 集成测试

按 `AGENTS.md` 规范，example 必须配 VDP 集成测试，路径 `examples/aoi-demo/tests/vdp_aoi.rs`。

测试设计要点：
- 通过 `game.setState` 把圆**钉死在固定位置**（消除动画时序 race）
- 同时验证 **AOI 层**（`aoi.queryCircle` 返回值）和 **业务层**（散点是否真的变色）
- 测一个 `enter → leave` 完整循环，对称性自动覆盖

---

## 测试策略

按 `AGENTS.md` 的硬性要求：

### 1. `vibe_aoi` 内部单元测试（必须）

- `BruteForce` 和 `UniformGrid` 用**同一组黄金用例**跑——保证两个后端结果完全一致
- 边界 case：空 world、单实体、形状跨多个 cell、查询完全在 world 之外
- `Observer` 的 enter / leave 事件去重（同一帧多次满足条件不应重复触发 enter）
- `Shape` 几何判定的对称性（`a ∩ b == b ∩ a`）
- VDP 序列化往返（`vdp` feature 下）

### 2. Example 集成测试

`examples/aoi-demo/tests/vdp_aoi.rs`，作为 P5 阶段的最终验收。

### 3. 验证命令

| 改动范围 | 命令 |
|---------|------|
| `vibe_aoi` 本身 | `cargo test -p vibe_aoi` |
| `examples/aoi-demo` | `cargo test -p aoi-demo -- --ignored --test-threads=1` |
| Feature 剥离路径 | `cargo build --no-default-features` |

---

## 实施路径

按 `AGENTS.md` 的 todo list 规范，分 6 个阶段推进。每阶段必须**跑通对应测试**才能算完成。

| 阶段 | 内容 | 验收 |
|------|------|------|
| **P1** | `vibe_aoi` crate 骨架：`Shape` / `EntityId` / `AoiWorld` / `BruteForce` 后端 + 单测 | `cargo test -p vibe_aoi` 全绿 |
| **P2** | `UniformGrid` 后端 + 双后端一致性测试 | 同上 |
| **P3** | `Observer` enter / leave 事件 + `raycast` | 同上 |
| **P4** | `vdp` feature + `AoiWorld::handle_vdp` helper + JSON 序列化 | 单测覆盖 JSON 路由 |
| **P5** | `examples/aoi-demo`（多圆 + 散点 + 鼠标 hover + stats）+ VDP 集成测试 | `cargo test -p aoi-demo -- --ignored --test-threads=1` |
| **P6** | 文档同步：`docs/api.md` 加 AOI 章节、`AGENTS.md` 仓库结构 + crate 依赖图、本文档收尾 | `cargo fmt --all -- --check` 通过 |

每个阶段完成后立即更新 todo list 状态；P5 启动前必须确认 P1~P4 都已 `cargo test` 通过。

---

## 命名规范

遵守 `AGENTS.md` 中的全局命名规范：

| 类别 | 规范 | 示例 |
|------|------|------|
| Crate | `snake_case` | `vibe_aoi` |
| 结构体 / 枚举 | `PascalCase` | `AoiWorld`、`Shape`、`AoiEvent` |
| 函数 | `snake_case` | `query_circle`、`drain_events` |
| VDP 方法 | `namespace.camelCase` | `aoi.queryCircle`、`aoi.list` |
| Feature 名 | 全 workspace 统一 `vdp` | `vibe_aoi/vdp` |

---

## Filter / LOD（已实现）

> **场景**：游戏经常需要在 AOI 之上叠一层「按类型分组」「按距离 LOD」的过滤逻辑，而这些逻辑本身**不属于** AOI 库的核心职责。如果硬塞进 `Shape` 或 broadphase，库就会变成游戏胶水。Vibe2D 的方案是把过滤抽象成一个**用户提供的闭包**，库只负责在 broadphase 之后调用它。

### 类型签名

```rust
pub type AoiFilter = dyn Fn(EntityId, &Shape, &Shape) -> bool + Send + Sync;
```

参数依次是：
1. `EntityId` — 候选实体的句柄。游戏侧通常用它去查自己维护的"类型表"（如 `HashMap<EntityId, Faction>`）。
2. `&Shape` — 候选实体的 AOI 形状。
3. `&Shape` — 一次性查询时是 query region；observer 上挂的过滤器收到的是 observer region。**距离 LOD 必须用这个参数**：从两个 shape 的 `aabb_bounds()` 中点算出距离，再判断是否在 LOD 半径内。

`Send + Sync` 是硬性要求 —— `AoiWorld` 没有把自己钉死在主线程，未来工作线程化时不需要再改 API。

### 一次性 query 的过滤版本

```rust
fn query_aabb_filtered<F>(&mut self, min: Vec2, max: Vec2, filter: F) -> Vec<EntityId>
fn query_circle_filtered<F>(&mut self, center: Vec2, radius: f32, filter: F) -> Vec<EntityId>
fn query_point_filtered<F>(&mut self, p: Vec2, filter: F) -> Vec<EntityId>
fn raycast_filtered<F>(&self, origin: Vec2, dir: Vec2, max_dist: f32, filter: F) -> Option<RaycastHit>
// 其中 F: Fn(EntityId, &Shape, &Shape) -> bool
```

实现路径：先走 backend 的 broadphase 拿候选 → 再用闭包过滤。这意味着 **filter 不会减少 broadphase 的工作量，只减少返回结果集大小**（以及游戏侧后续处理的成本）。如果你需要"少数 LOD 区段 + 大部分跳过"的场景，应该收紧 `radius` / `max` 而不是依赖 filter 减负。

### Observer 上的持久化 filter

```rust
fn create_observer_filtered<F>(&mut self, region: Shape, filter: F) -> ObserverId
fn set_observer_filter<F>(&mut self, id: ObserverId, filter: Option<F>)
// 其中 F: Fn(EntityId, &Shape, &Shape) -> bool + Send + Sync + 'static
```

Filter 是 observer **持久状态的一部分**，存储为 `Box<dyn Fn>`。`update_observer` 仍然只接 region —— 移动 observer 时 filter 自动复用。

#### 关键不变量：filter 改变 → diff 基线改变 → 触发 Enter/Leave

这是设计上最容易踩坑的一点，必须讲清楚：

1. observer 的 `current` hit set 永远是「**当前 filter 接受**的实体」的集合。
2. `set_observer_filter(id, new_filter)` **只**替换 filter，**不**立即重新查询。
3. 下一次 `update_observer(id, region)` 时：
   - 用**新** filter 重新计算应该在 set 里的实体；
   - 与旧 set 做 diff，**为新 filter 接受但旧 filter 拒绝的实体发 Enter，反之发 Leave**。

**这正是 LOD 系统想要的语义**：当玩家收紧 LOD 半径时，超出新半径的实体确实「**对这个 observer 来说**消失了」，发 Leave 让网络层把它们从客户端 drop 掉。Vibe2D 不把这个语义叫 bug，而是当作正确行为暴露出来。

#### Send + Sync + 'static 的代价：闭包不能借用游戏数据

因为 filter 要存进 observer，必须 `'static`，所以闭包**不能借用** game 的 `&self`、`&HashMap` 等。两个常见解决办法：

- **`Arc<HashMap>` 共享只读表**：游戏在启动时建一张 `Arc<HashMap<EntityId, Kind>>`，每个 observer 的 filter 闭包持一个 `Arc::clone`。这是 `examples/aoi-demo` 用的模式。
- **`Arc<RwLock<…>>` 用于可变状态**：如果 filter 要根据运行时状态判断（比如阵营关系会变），用 `RwLock` 包起来再 clone Arc。注意每次 filter 调用都要拿读锁，注意热路径开销。

### 完整示例：类型 + 距离双重过滤（出自 `examples/aoi-demo`）

```rust
// 1. 散点分两类，存类型表（Arc 是为了让 filter 闭包能持 'static 引用）
let scatter_kind: Arc<HashMap<EntityId, ScatterKind>> = Arc::new(/* ... */);
let lod_enabled = false;

// 2. 工厂函数生成 filter，把当前 LOD 状态闭包捕获进去
fn build_filter(
    kind: Arc<HashMap<EntityId, ScatterKind>>,
    lod_enabled: bool,
) -> impl Fn(EntityId, &Shape, &Shape) -> bool + Send + Sync + 'static {
    move |id, entity_shape, observer_region| {
        // 类型过滤：只接受 Round
        if kind.get(&id).copied() != Some(ScatterKind::Round) {
            return false;
        }
        // 距离 LOD：可选
        if !lod_enabled {
            return true;
        }
        let (e_min, e_max) = entity_shape.aabb_bounds();
        let entity_center = (e_min + e_max) * 0.5;
        let (r_min, r_max) = observer_region.aabb_bounds();
        let region_center = (r_min + r_max) * 0.5;
        entity_center.distance(region_center) < LOD_RADIUS
    }
}

// 3. 创建带 filter 的 observer
let observer = aoi.create_observer_filtered(
    Shape::circle(pos, CIRCLE_RADIUS),
    build_filter(scatter_kind.clone(), lod_enabled),
);

// 4. 玩家按 [L] 切换 LOD：换一个新闭包，下一次 update_observer 自动 diff
fn on_lod_toggle(world: &mut AoiWorld, ...) {
    lod_enabled = !lod_enabled;
    for c in &circles {
        world.set_observer_filter(
            c.observer,
            Some(build_filter(scatter_kind.clone(), lod_enabled)),
        );
    }
}
```

### 故意**不**做的设计

- **VDP 不暴露 filter**：闭包没法序列化跨进程。`aoi.queryCircle` 等 VDP 方法仍然是无过滤版本 —— 这也是 demo 集成测试里要"再走一遍 `aoi.list` 取出 shape 类型，自己在客户端侧过滤"的原因。如果要远程调试 filter 行为，建议在游戏的 `inspect()` 里把 filter 后的视图额外 dump 出来。
- **filter 不参与 broadphase**：如前所述，filter 只缩小返回集，不改变 grid 查询的 cell 数。如果性能瓶颈在 broadphase 自身（比如 LOD 半径远小于 observer 半径），应该再开一个**小**半径的查询，而不是依赖 filter。
- **没有 mask / layer**：故意不引入位掩码层级系统。`Fn` 闭包覆盖 mask 能做的所有事，并且不需要在 API 里腾出 16 / 32 个 bit 给 layer ID。当未来如果 mask 模式真的高频出现，可以加一层 `query_*_with_mask` 语法糖，内部仍然走 filter 路径。

---

## 未来扩展（不在第一版范围内）

记录下来作为后续路标：

- **Hash Grid 后端**：当世界变得无界且实体稀疏时（如 Minecraft chunk 索引）
- **`Layer` / `mask` 系统**：让查询能区分「玩家 vs 敌人 vs 道具」（参考 Unity 的 LayerMask）
- **大物体专用层**：差异巨大的实体大小可以分层（小实体走 grid、大实体走单独列表）
- **`vibe_physics` 复用**：当 physics 真正实现时，让它依赖 `vibe_aoi` 做 broadphase
- **引擎级集成**：如果证明价值足够大，把 AoiWorld 升格成 `Context.aoi`，提供引擎级的 VDP 内省和调试器可视化
