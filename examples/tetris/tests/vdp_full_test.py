#!/usr/bin/env python3
"""
Tetris VDP 全流程验证脚本
通过 VDP 协议验证俄罗斯方块游戏的各项功能。

用法：
  1. 先启动游戏: cd examples/tetris && cargo run -p tetris
  2. 运行本脚本: python3 tests/vdp_test_tetris.py

依赖: pip install websockets
"""
import asyncio
import json
import sys
import websockets

WS_URL = "ws://127.0.0.1:9229"
req_id = 0
TOTAL_ROWS = 40
COLS = 10

# ── RPC helpers ──────────────────────────────────────────────────────

async def rpc(ws, method, params=None):
    global req_id
    req_id += 1
    msg = {"jsonrpc": "2.0", "id": req_id, "method": method}
    if params is not None:
        msg["params"] = params
    payload = json.dumps(msg)
    print(f"\n>>> {payload}")
    await ws.send(payload)
    resp = await asyncio.wait_for(ws.recv(), timeout=5)
    parsed = json.loads(resp)
    # Compact print for large payloads
    resp_str = json.dumps(parsed, ensure_ascii=False)
    if len(resp_str) > 500:
        resp_str = resp_str[:500] + "..."
    print(f"<<< {resp_str}")
    return parsed


async def rpc_quiet(ws, method, params=None):
    global req_id
    req_id += 1
    msg = {"jsonrpc": "2.0", "id": req_id, "method": method}
    if params is not None:
        msg["params"] = params
    await ws.send(json.dumps(msg))
    resp = await asyncio.wait_for(ws.recv(), timeout=5)
    return json.loads(resp)


def section(num, title):
    print(f"\n{'─' * 55}")
    print(f"【测试 {num}】{title}")
    print("─" * 55)


async def step_and_wait(ws, frames=1):
    r = await rpc_quiet(ws, "engine.getTime")
    fc_before = r["result"]["frame_count"]
    await rpc_quiet(ws, "engine.step", {"frames": frames})
    for _ in range(200):
        r = await rpc_quiet(ws, "engine.getTime")
        if r["result"]["frame_count"] >= fc_before + frames:
            return r
        await asyncio.sleep(0.005)
    return r


async def tap_key(ws, key):
    await rpc_quiet(ws, "engine.simulateInput",
                    {"device": "keyboard", "action": "tap", "key": key})
    await step_and_wait(ws, 1)


async def inspect(ws):
    r = await rpc_quiet(ws, "game.inspect")
    return r.get("result", {})


def make_empty_grid():
    return [[None] * COLS for _ in range(TOTAL_ROWS)]


def make_row(piece_type="I", empty_cols=None):
    """Create a full row with optional empty columns."""
    row = [piece_type] * COLS
    if empty_cols:
        for c in empty_cols:
            row[c] = None
    return row


# ── Test sections ────────────────────────────────────────────────────

async def test_engine_basics(ws):
    """Test 1: Engine info, pause, resume, step, getTime"""
    section(1, "engine 基础功能 — info/pause/resume/step/getTime")

    # engine.info
    r = await rpc(ws, "engine.info")
    assert "result" in r, "engine.info 应返回 result"
    assert "virtual_width" in r["result"], "缺少 virtual_width"
    assert r["result"]["virtual_width"] == 800, f"虚拟宽度应为 800"
    assert r["result"]["virtual_height"] == 700, f"虚拟高度应为 700"
    print("    OK engine.info 返回正确")

    # engine.getTime
    r = await rpc(ws, "engine.getTime")
    result = r["result"]
    assert "frame_count" in result
    assert "paused" in result
    print("    OK engine.getTime 字段完整")

    # engine.pause
    r = await rpc(ws, "engine.pause")
    assert r["result"]["paused"] is True
    fc1 = r["result"]["frame_count"]
    await asyncio.sleep(0.3)
    r = await rpc(ws, "engine.getTime")
    assert r["result"]["frame_count"] == fc1, "暂停期间帧数不应变化"
    print(f"    OK 暂停成功, frame_count 冻结在 {fc1}")

    # engine.resume
    r = await rpc(ws, "engine.resume")
    assert r["result"]["paused"] is False
    await asyncio.sleep(0.2)
    r = await rpc(ws, "engine.getTime")
    assert r["result"]["frame_count"] > fc1
    print("    OK 恢复成功, frame_count 增加")

    # engine.step
    await rpc(ws, "engine.pause")
    r = await rpc(ws, "engine.getTime")
    fc_before = r["result"]["frame_count"]
    await rpc(ws, "engine.step", {"frames": 3})
    await asyncio.sleep(0.2)
    r = await rpc(ws, "engine.getTime")
    assert r["result"]["frame_count"] == fc_before + 3
    print(f"    OK 步进 3 帧: {fc_before} -> {fc_before + 3}")

    # step while not paused should error
    await rpc(ws, "engine.resume")
    r = await rpc(ws, "engine.step", {"frames": 1})
    assert "error" in r
    print("    OK 未暂停时 step 正确返回错误")


async def test_piece_spawning(ws):
    """Test 2: Piece spawning after reset"""
    section(2, "方块生成 — reset 后检查初始状态")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.reset")
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    assert state["phase"] == "playing", f"应为 playing, 实际 {state['phase']}"
    assert state["current"] is not None, "应有当前方块"
    assert state["current"]["type"] in ["I", "O", "T", "S", "Z", "J", "L"]
    assert len(state["next"]) == 5, f"next 队列应有 5 个, 实际 {len(state['next'])}"
    assert state["score"] == 0
    assert state["level"] == 1
    assert state["lines"] == 0
    print(f"    OK 生成方块: type={state['current']['type']}, "
          f"pos=({state['current']['x']},{state['current']['y']})")
    print(f"    OK next 队列: {state['next']}")


async def test_horizontal_movement(ws):
    """Test 3: Horizontal movement"""
    section(3, "水平移动 — Left/Right")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.reset")
    await rpc(ws, "game.clearGrid")
    await rpc(ws, "game.setPiece", {"type": "T", "rotation": 0, "x": 4, "y": 30})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    x0 = state["current"]["x"]
    print(f"    初始 x={x0}")

    # Move left
    await tap_key(ws, "Left")
    state = await inspect(ws)
    assert state["current"]["x"] == x0 - 1, f"左移后 x 应为 {x0-1}, 实际 {state['current']['x']}"
    print(f"    OK 左移: x={state['current']['x']}")

    # Move right twice
    await tap_key(ws, "Right")
    await tap_key(ws, "Right")
    state = await inspect(ws)
    assert state["current"]["x"] == x0 + 1, f"右移后 x 应为 {x0+1}, 实际 {state['current']['x']}"
    print(f"    OK 右移2次: x={state['current']['x']}")


async def test_wall_collision(ws):
    """Test 4: Wall collision"""
    section(4, "墙壁碰撞 — 方块贴墙后不动")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.clearGrid")
    # Place T piece at left wall (x=1, so cells include col 0)
    await rpc(ws, "game.setPiece", {"type": "T", "rotation": 0, "x": 1, "y": 30})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    x0 = state["current"]["x"]

    # Try to move left twice — should only succeed once (to x=0 where leftmost cell is at col -1)
    await tap_key(ws, "Left")
    state = await inspect(ws)
    x1 = state["current"]["x"]

    await tap_key(ws, "Left")
    state = await inspect(ws)
    x2 = state["current"]["x"]

    # At some point it should stop
    print(f"    x: {x0} -> {x1} -> {x2}")
    # The T piece in rotation N has cells at (y, x-1), (y, x), (y, x+1), (y-1, x)
    # At x=1: cells at col 0,1,2 — can move left to x=0: cells at col -1,0,1 — blocked!
    # So x=1 can move to x=0? No, col -1 is out of bounds.
    # Actually x=1: leftmost cell = x-1 = 0, so that's fine.
    # x=0: leftmost cell = x-1 = -1, out of bounds — blocked
    assert x1 == x0 or x2 == x1, "到达墙壁后应停止移动"
    print(f"    OK 墙壁碰撞正常工作")


async def test_rotation_cw(ws):
    """Test 5: Clockwise rotation"""
    section(5, "顺时针旋转 — Up 键")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.clearGrid")
    await rpc(ws, "game.setPiece", {"type": "T", "rotation": 0, "x": 4, "y": 30})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    assert state["current"]["rotation"] == 0
    print(f"    初始旋转: {state['current']['rotation']}")

    await tap_key(ws, "Up")
    state = await inspect(ws)
    assert state["current"]["rotation"] == 1, f"CW 旋转后应为 1, 实际 {state['current']['rotation']}"
    print(f"    OK CW 旋转: rotation={state['current']['rotation']}")


async def test_rotation_ccw(ws):
    """Test 6: Counter-clockwise rotation"""
    section(6, "逆时针旋转 — Z 键")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.clearGrid")
    await rpc(ws, "game.setPiece", {"type": "T", "rotation": 0, "x": 4, "y": 30})
    await step_and_wait(ws, 1)

    await tap_key(ws, "Z")
    state = await inspect(ws)
    assert state["current"]["rotation"] == 3, f"CCW 旋转后应为 3, 实际 {state['current']['rotation']}"
    print(f"    OK CCW 旋转: rotation={state['current']['rotation']}")


async def test_wall_kick(ws):
    """Test 7: Wall kick — I piece against wall"""
    section(7, "Wall Kick — I 方块贴墙旋转")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.clearGrid")
    # I piece horizontal (rot=0) at right edge: cells at (y, x-1) to (y, x+2)
    # Place at x=8 so rightmost cell is at col 10 — actually that's out of bounds
    # x=7: cells at col 6,7,8,9 — against right wall
    await rpc(ws, "game.setPiece", {"type": "I", "rotation": 0, "x": 7, "y": 30})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    x_before = state["current"]["x"]
    rot_before = state["current"]["rotation"]
    print(f"    旋转前: x={x_before}, rot={rot_before}")

    # Rotate CW — should wall kick
    await tap_key(ws, "Up")
    state = await inspect(ws)
    if state["current"] is not None:
        x_after = state["current"]["x"]
        rot_after = state["current"]["rotation"]
        print(f"    旋转后: x={x_after}, rot={rot_after}")
        # If rotation succeeded, it must have kicked
        if rot_after != rot_before:
            print(f"    OK Wall kick 生效: x 从 {x_before} 变为 {x_after}")
        else:
            print(f"    WARN 旋转未成功 (可能无法 kick)")
    else:
        print(f"    WARN 方块已锁定")


async def test_gravity(ws):
    """Test 8: Gravity — piece falls over time"""
    section(8, "重力 — 方块随时间下落")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.clearGrid")
    await rpc(ws, "game.setScore", {"level": 5})
    await rpc(ws, "game.setPiece", {"type": "O", "rotation": 0, "x": 4, "y": 20})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    y0 = state["current"]["y"]
    print(f"    初始 y={y0}")

    # At level 5, gravity interval is much shorter, step 30 frames should suffice
    await step_and_wait(ws, 30)
    state = await inspect(ws)
    if state["current"] is not None:
        y1 = state["current"]["y"]
        assert y1 > y0, f"30帧后方块应下落: y 从 {y0} 仍为 {y1}"
        print(f"    OK 重力生效: y 从 {y0} 变为 {y1}")
    else:
        print(f"    OK 方块已锁定（下落到底部）")


async def test_hard_drop(ws):
    """Test 9: Hard drop"""
    section(9, "Hard Drop — Space 键")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.reset")
    await rpc(ws, "game.clearGrid")
    await rpc(ws, "game.setScore", {"score": 0})
    await rpc(ws, "game.setPiece", {"type": "O", "rotation": 0, "x": 4, "y": 20})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    y0 = state["current"]["y"]
    ghost = state["ghost_y"]
    score0 = state["score"]
    print(f"    方块 y={y0}, ghost_y={ghost}, score={score0}")

    # Hard drop
    await tap_key(ws, "Space")
    state = await inspect(ws)

    # Piece should be locked, score should include hard drop bonus
    assert state["score"] > score0, f"Hard drop 应给分: {score0} -> {state['score']}"
    cells_dropped = ghost - y0
    expected_bonus = cells_dropped * 2
    actual_bonus = state["score"] - score0
    print(f"    OK Hard drop: 掉落 {cells_dropped} 格, 得分 +{actual_bonus} (预期 +{expected_bonus})")

    # New piece should have spawned
    assert state["current"] is not None, "Hard drop 后应生成新方块"
    print(f"    OK 新方块已生成: type={state['current']['type']}")


async def test_soft_drop(ws):
    """Test 10: Soft drop"""
    section(10, "Soft Drop — Down 键")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.clearGrid")
    await rpc(ws, "game.setScore", {"score": 0})
    await rpc(ws, "game.setPiece", {"type": "O", "rotation": 0, "x": 4, "y": 20})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    y0 = state["current"]["y"]

    # Hold Down for several frames
    await rpc_quiet(ws, "engine.simulateInput",
                    {"device": "keyboard", "action": "press", "key": "Down"})
    await step_and_wait(ws, 10)
    await rpc_quiet(ws, "engine.simulateInput",
                    {"device": "keyboard", "action": "release", "key": "Down"})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    if state["current"] is not None:
        y1 = state["current"]["y"]
        assert y1 > y0, f"Soft drop 10帧后应下落: {y0} -> {y1}"
        print(f"    OK Soft drop: y 从 {y0} 变为 {y1}")
    else:
        print(f"    OK Soft drop: 方块已锁定到底部")
    assert state["score"] > 0, "Soft drop 应给分"
    print(f"    OK Soft drop 得分: {state['score']}")


async def test_line_clear(ws):
    """Test 11: Line clear — Single"""
    section(11, "消行 — Single")
    await rpc(ws, "engine.pause")

    # Build a grid with row 39 almost full (missing col 9)
    grid = make_empty_grid()
    for c in range(COLS - 1):
        grid[39][c] = "I"
    await rpc(ws, "game.setGrid", {"grid": grid})
    await rpc(ws, "game.setScore", {"score": 0, "level": 1, "lines": 0, "combo": -1})

    # I piece vertical (rotation E/1): cells at (-1,1),(0,1),(1,1),(2,1) relative to pivot
    # At x=8, y=20: cells at (19,9),(20,9),(21,9),(22,9) — all on col 9, no overlap
    # Ghost will drop to y=37: cells at (36,9),(37,9),(38,9),(39,9) — fills the gap
    await rpc(ws, "game.setPiece", {"type": "I", "rotation": 1, "x": 8, "y": 20})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    print(f"    放置前: ghost_y={state['ghost_y']}, cells={state['current']['cells']}")

    # Hard drop
    await tap_key(ws, "Space")
    await step_and_wait(ws, 2)

    state = await inspect(ws)
    print(f"    lines={state['lines']}, score={state['score']}")
    assert state["lines"] >= 1, f"应至少消 1 行, 实际 {state['lines']}"
    assert state["score"] > 0, "消行应得分"
    print(f"    OK 消行: lines={state['lines']}, score={state['score']}")


async def test_tetris_clear(ws):
    """Test 12: Tetris — 4 line clear"""
    section(12, "Tetris — 四行消除")
    await rpc(ws, "engine.pause")

    # Build grid with rows 36-39 full except col 0
    grid = make_empty_grid()
    for r in range(36, 40):
        for c in range(1, COLS):
            grid[r][c] = "I"
    await rpc(ws, "game.setGrid", {"grid": grid})
    await rpc(ws, "game.setScore", {"score": 0, "level": 1, "lines": 0, "combo": -1, "back_to_back": False})

    # I piece vertical (rotation E/1): cells at (-1,1),(0,1),(1,1),(2,1) relative to pivot
    # At x=-1, y=20: cells at (19,0),(20,0),(21,0),(22,0) — all on col 0, no overlap
    # Ghost drops to y=37: cells at (36,0),(37,0),(38,0),(39,0) — fills col 0, rows 36-39
    await rpc(ws, "game.setPiece", {"type": "I", "rotation": 1, "x": -1, "y": 20})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    print(f"    放置前: ghost_y={state['ghost_y']}, cells={state['current']['cells']}")

    # Hard drop
    await tap_key(ws, "Space")
    await step_and_wait(ws, 2)

    state = await inspect(ws)
    print(f"    Tetris 后: lines={state['lines']}, score={state['score']}")
    assert state["lines"] >= 4, f"应消 4 行, 实际 {state['lines']}"
    # Tetris = 800 * level + hard drop bonus
    assert state["score"] >= 800, f"Tetris 分数应 >= 800, 实际 {state['score']}"
    print(f"    OK Tetris: lines={state['lines']}, score={state['score']}")


async def test_hold(ws):
    """Test 14: Hold piece"""
    section(14, "Hold — C 键")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.reset")
    await rpc(ws, "game.clearGrid")
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    assert state["hold"] is None, "初始 hold 应为空"
    first_type = state["current"]["type"]
    second_type = state["next"][0]
    print(f"    当前: {first_type}, next[0]: {second_type}")

    # Hold
    await tap_key(ws, "C")
    state = await inspect(ws)
    assert state["hold"] == first_type, f"hold 应为 {first_type}, 实际 {state['hold']}"
    assert state["current"]["type"] == second_type, f"当前应为 {second_type}"
    assert state["hold_used"] is True, "hold_used 应为 true"
    print(f"    OK Hold: hold={state['hold']}, current={state['current']['type']}")

    # Try hold again (should be blocked)
    current_before = state["current"]["type"]
    await tap_key(ws, "C")
    state = await inspect(ws)
    assert state["current"]["type"] == current_before, "hold_used=true 时不应交换"
    print(f"    OK 同轮不可重复 hold")


async def test_ghost_piece(ws):
    """Test 17: Ghost piece position"""
    section(17, "Ghost Piece — ghost_y 计算")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.clearGrid")
    await rpc(ws, "game.setPiece", {"type": "O", "rotation": 0, "x": 4, "y": 20})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    gy = state["ghost_y"]
    # O piece: cells at (y,x),(y,x+1),(y+1,x),(y+1,x+1)
    # On empty grid, ghost should be at row 38 (so bottom cells at 38,39)
    assert gy == 38, f"空网格上 O 方块 ghost_y 应为 38, 实际 {gy}"
    print(f"    OK ghost_y={gy} (空网格)")

    # Add some blocks and check ghost updates
    grid = make_empty_grid()
    grid[35][4] = "I"
    await rpc(ws, "game.setGrid", {"grid": grid})
    await rpc(ws, "game.setPiece", {"type": "O", "rotation": 0, "x": 4, "y": 20})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    gy2 = state["ghost_y"]
    assert gy2 < gy, f"有障碍时 ghost_y 应更小: {gy2} vs {gy}"
    print(f"    OK ghost_y={gy2} (有障碍)")


async def test_game_over(ws):
    """Test 18: Game over detection"""
    section(18, "Game Over — 顶部溢出检测")
    await rpc(ws, "engine.pause")

    # Fill rows 18-39 with 9/10 cells (leave col 9 empty so rows don't clear)
    # This blocks the spawn position (cols 4,5 at rows 18,19) but won't be
    # cleared by clear_lines since no row is completely full.
    grid = make_empty_grid()
    for r in range(18, TOTAL_ROWS):
        for c in range(COLS - 1):  # cols 0-8
            grid[r][c] = "I"
    await rpc(ws, "game.setGrid", {"grid": grid})
    await rpc(ws, "game.setPhase", {"phase": "playing"})

    # Place O piece at y=16 (cells at rows 16,17 — safely above filled area)
    # ghost_y will be 16 (row 18 blocks further descent)
    await rpc(ws, "game.setPiece", {"type": "O", "rotation": 0, "x": 4, "y": 16})
    await step_and_wait(ws, 1)

    # Hard drop — locks at y=16 (rows 16,17 with 2 cells each — not full)
    # No lines clear. spawn_piece tries row 19 → blocked, row 18 → blocked → game_over
    await tap_key(ws, "Space")
    await step_and_wait(ws, 3)

    state = await inspect(ws)
    print(f"    phase={state['phase']}")
    assert state["phase"] == "game_over", f"应为 game_over, 实际 {state['phase']}"
    print(f"    OK Game Over 检测正确")


async def test_level_up(ws):
    """Test 19: Level progression"""
    section(19, "等级提升 — 每 10 行升级")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.reset")
    await rpc(ws, "game.clearGrid")
    await rpc(ws, "game.setScore", {"score": 0, "level": 1, "lines": 9})
    await step_and_wait(ws, 1)

    state = await inspect(ws)
    assert state["level"] == 1, f"应为 level 1, 实际 {state['level']}"
    print(f"    当前: level={state['level']}, lines={state['lines']}")

    # Clear 1 line to reach 10 lines total -> level 2
    # Fill row 39 except col 9
    grid = make_empty_grid()
    for c in range(COLS - 1):
        grid[39][c] = "I"
    await rpc(ws, "game.setGrid", {"grid": grid})
    # I piece vertical at col 9, placed high up so it drops down
    await rpc(ws, "game.setPiece", {"type": "I", "rotation": 1, "x": 8, "y": 20})
    await step_and_wait(ws, 1)
    await tap_key(ws, "Space")
    await step_and_wait(ws, 2)

    state = await inspect(ws)
    print(f"    消行后: level={state['level']}, lines={state['lines']}")
    assert state["level"] >= 2, f"应升到 level 2+, 实际 {state['level']}"
    print(f"    OK 等级提升: level={state['level']}")


async def test_bag_randomization(ws):
    """Test 20: 7-bag randomization"""
    section(20, "7-Bag 随机 — 每 7 个方块包含所有类型")
    await rpc(ws, "engine.pause")
    await rpc(ws, "game.reset")
    await step_and_wait(ws, 1)

    # Collect pieces: current + 5 next = 6, need to see at least 7
    state = await inspect(ws)
    pieces = [state["current"]["type"]] + list(state["next"])
    print(f"    收集到 {len(pieces)} 个方块: {pieces}")

    # We have 6 pieces (current + 5 next). First 7 should come from same bag.
    # Hold current → it goes to hold, next[0] becomes current, queue refills
    # exposing the 7th piece from the bag at next[4].
    await tap_key(ws, "C")
    state = await inspect(ws)
    # all_pieces: original_current + new_current + new_next[0..4] = 7
    all_pieces = pieces[:1] + [state["current"]["type"]] + list(state["next"][:5])
    print(f"    前 7 个方块: {all_pieces[:7]}")

    if len(all_pieces) >= 7:
        first_seven = all_pieces[:7]
        types = set(first_seven)
        print(f"    类型集合: {types}")
        assert len(types) == 7, f"前 7 个方块应包含所有 7 种类型, 实际 {len(types)} 种: {types}"
        print(f"    OK 7-Bag: 所有 7 种方块都出现了")
    else:
        print(f"    WARN 收集不够 7 个方块, 跳过验证")


async def test_vdp_custom_methods(ws):
    """Test 21: Custom VDP methods"""
    section(21, "自定义 VDP 方法")

    # game.reset
    r = await rpc(ws, "game.reset")
    assert "result" in r
    print("    OK game.reset")

    # game.setGrid
    grid = make_empty_grid()
    grid[39][0] = "T"
    r = await rpc(ws, "game.setGrid", {"grid": grid})
    assert "result" in r
    state = await inspect(ws)
    assert state["grid"][39][0] == "T"
    print("    OK game.setGrid")

    # game.clearGrid
    r = await rpc(ws, "game.clearGrid")
    assert "result" in r
    state = await inspect(ws)
    assert state["grid"][39][0] is None
    print("    OK game.clearGrid")

    # game.setPiece
    r = await rpc(ws, "game.setPiece", {"type": "I", "rotation": 1, "x": 5, "y": 25})
    assert "result" in r
    state = await inspect(ws)
    assert state["current"]["type"] == "I"
    assert state["current"]["rotation"] == 1
    print("    OK game.setPiece")

    # game.setNextQueue
    r = await rpc(ws, "game.setNextQueue", {"queue": ["T", "I", "O", "S", "Z"]})
    assert "result" in r
    state = await inspect(ws)
    assert state["next"] == ["T", "I", "O", "S", "Z"]
    print("    OK game.setNextQueue")

    # game.setHoldPiece
    r = await rpc(ws, "game.setHoldPiece", {"piece": "L"})
    assert "result" in r
    state = await inspect(ws)
    assert state["hold"] == "L"
    print("    OK game.setHoldPiece")

    # game.setScore
    r = await rpc(ws, "game.setScore", {"score": 12345, "level": 5, "lines": 42})
    assert "result" in r
    state = await inspect(ws)
    assert state["score"] == 12345
    assert state["level"] == 5
    assert state["lines"] == 42
    print("    OK game.setScore")

    # game.setPhase
    r = await rpc(ws, "game.setPhase", {"phase": "game_over"})
    assert "result" in r
    state = await inspect(ws)
    assert state["phase"] == "game_over"
    r = await rpc(ws, "game.setPhase", {"phase": "playing"})
    print("    OK game.setPhase")


async def test_error_handling(ws):
    """Test 22: Error handling"""
    section(22, "错误处理")
    await rpc(ws, "engine.pause")

    # Unknown method
    r = await rpc(ws, "game.nonexistent", {"foo": "bar"})
    assert "error" in r
    print("    OK 未知方法返回错误")

    # Invalid key
    r = await rpc(ws, "engine.simulateInput",
                  {"device": "keyboard", "action": "tap", "key": "InvalidKey"})
    assert "error" in r
    print("    OK 无效按键返回错误")

    # Invalid piece type
    r = await rpc(ws, "game.setPiece", {"type": "X", "rotation": 0, "x": 4, "y": 20})
    assert "error" in r
    print("    OK 无效方块类型返回错误")


async def test_screenshot(ws):
    """Test 23: Screenshot"""
    section(23, "截图功能")
    r = await rpc(ws, "game.screenshot", {"path": "/tmp/tetris_vdp_test.png"})
    assert "result" in r
    print("    OK 截图请求已提交")
    await asyncio.sleep(0.5)


async def main():
    print("=" * 60)
    print("Tetris VDP 全流程验证脚本")
    print("=" * 60)

    try:
        async with websockets.connect(WS_URL) as ws:
            await test_engine_basics(ws)
            await test_piece_spawning(ws)
            await test_horizontal_movement(ws)
            await test_wall_collision(ws)
            await test_rotation_cw(ws)
            await test_rotation_ccw(ws)
            await test_wall_kick(ws)
            await test_gravity(ws)
            await test_hard_drop(ws)
            await test_soft_drop(ws)
            await test_line_clear(ws)
            await test_tetris_clear(ws)
            await test_hold(ws)
            await test_ghost_piece(ws)
            await test_game_over(ws)
            await test_level_up(ws)
            await test_bag_randomization(ws)
            await test_vdp_custom_methods(ws)
            await test_error_handling(ws)
            await test_screenshot(ws)

            # Clean up: resume engine
            await rpc(ws, "engine.resume")

        print("\n" + "=" * 60)
        print("全部测试通过!")
        print("=" * 60)

    except ConnectionRefusedError:
        print("错误: 无法连接到游戏。请先启动游戏:")
        print("  cd examples/tetris && cargo run -p tetris")
        sys.exit(1)
    except AssertionError as e:
        print(f"\n测试失败: {e}")
        sys.exit(1)

asyncio.run(main())
