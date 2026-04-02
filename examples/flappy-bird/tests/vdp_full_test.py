#!/usr/bin/env python3
"""
Vibe2D VDP 全流程验证脚本
通过 VDP 协议自动操控 Flappy Bird，完成从启动到得分再到死亡的完整流程。

用法：
  1. 先启动游戏: cd examples/flappy-bird && cargo run -p flappy-bird
  2. 运行本脚本: python3 tests/vdp_full_test.py

依赖: pip install websockets
"""
import asyncio
import json
import time
import websockets

WS_URL = "ws://127.0.0.1:9229"
req_id = 0

# ── 游戏常量（与 main.rs 对应）──
BIRD_X = 128.0
BIRD_W = 36.0
BIRD_H = 27.0
BIRD_LEFT = BIRD_X - BIRD_W / 2.0   # 110
PIPE_W = 50.0
PIPE_GAP = 70.0
TARGET_SCORE = 2


async def rpc(ws, method, params=None):
    """发送 JSON-RPC 请求并打印请求/响应。"""
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
    print(f"<<< {json.dumps(parsed, indent=2, ensure_ascii=False)}")
    return parsed


async def rpc_quiet(ws, method, params=None):
    """发送 JSON-RPC 请求，不打印（用于高频控制循环）。"""
    global req_id
    req_id += 1
    msg = {"jsonrpc": "2.0", "id": req_id, "method": method}
    if params is not None:
        msg["params"] = params
    await ws.send(json.dumps(msg))
    resp = await asyncio.wait_for(ws.recv(), timeout=5)
    return json.loads(resp)


def section(num, title):
    print(f"\n{'─' * 50}")
    print(f"【测试 {num}】{title}")
    print("─" * 50)


async def autopilot(ws):
    """
    自动操控小鸟穿越管道。
    策略：每 30ms 查询游戏状态，将小鸟 Y 坐标设置到最近管道间隙中心，
    使其安全通过。达到目标分数后停止操控，让小鸟自然死亡。
    """
    last_score = 0
    tick = 0
    default_y = 120.0  # 无管道时的安全高度

    print(f"    开始自动操控，目标分数: {TARGET_SCORE}")
    print(f"    策略: 每 30ms 通过 setBirdY(y, vy=0) 将小鸟定位到管道间隙中心")
    print()

    while True:
        r = await rpc_quiet(ws, "game.inspect")
        result = r.get("result", {})
        state = result.get("state", "")
        score = result.get("score", 0)
        bird = result.get("bird", {})
        pipes = result.get("pipes", [])

        # 游戏不在 playing 状态，退出
        if state != "playing":
            print(f"    [tick {tick}] 状态变为 '{state}'，退出控制循环")
            return score, state

        # 得分变化时打印
        if score != last_score:
            print(f"    [tick {tick}] ★ 得分变为 {score}！"
                  f"(bird_y={bird['y']:.1f}, vy={bird['vy']:.1f}, 管道数={len(pipes)})")
            last_score = score

        # 达到目标分数，停止操控
        if score >= TARGET_SCORE:
            print(f"    [tick {tick}] 已达到目标分数 {score}，停止操控，等待自然死亡...")
            return score, "playing"

        # 找到尚未通过的最近管道（pipe.x + PIPE_W > BIRD_LEFT）
        upcoming = [p for p in pipes if p["x"] + PIPE_W > BIRD_LEFT]

        if upcoming:
            upcoming.sort(key=lambda p: p["x"])
            nearest = upcoming[0]
            # 将小鸟中心对准间隙中心
            target_y = nearest["gap_y"] - BIRD_H / 2.0
            # 定期输出控制信息
            if tick % 30 == 0:
                print(f"    [tick {tick}] 管道 x={nearest['x']:.0f} "
                      f"gap_y={nearest['gap_y']:.0f} → bird_y 目标={target_y:.0f} "
                      f"(实际={bird['y']:.1f}, vy={bird['vy']:.1f})")
        else:
            target_y = default_y
            if tick % 30 == 0:
                print(f"    [tick {tick}] 无管道，保持 bird_y={target_y:.0f} "
                      f"(实际={bird['y']:.1f})")

        await rpc_quiet(ws, "game.setBirdY", {"y": target_y, "vy": 0})
        tick += 1
        await asyncio.sleep(0.03)


async def wait_for_death(ws, timeout=10.0):
    """停止操控后，等待小鸟自然死亡（落地或撞管道）。"""
    start = time.monotonic()
    while time.monotonic() - start < timeout:
        r = await rpc_quiet(ws, "game.inspect")
        state = r.get("result", {}).get("state", "")
        if state == "dead":
            return r
        await asyncio.sleep(0.05)
    return None


async def main():
    print("=" * 60)
    print("Vibe2D VDP 全流程验证（含自动游玩）")
    print("=" * 60)

    async with websockets.connect(WS_URL) as ws:
        # ━━━━━━━━ 阶段一：引擎信息 ━━━━━━━━
        section(1, "engine.info — 查询引擎基本信息")
        await rpc(ws, "engine.info")

        # ━━━━━━━━ 阶段二：初始状态 ━━━━━━━━
        section(2, "game.inspect — 查询当前游戏状态")
        await rpc(ws, "game.inspect")

        section(3, "game.setState → idle — 重置为待机状态")
        await rpc(ws, "game.setState", {"state": "idle"})

        section(4, "game.inspect — 验证 idle 状态")
        await rpc(ws, "game.inspect")

        # ━━━━━━━━ 阶段三：启动游戏 ━━━━━━━━
        section(5, "game.setState → countdown — 进入倒计时")
        await rpc(ws, "game.setState", {"state": "countdown"})

        section(6, "game.inspect — 验证 countdown 状态")
        r = await rpc(ws, "game.inspect")
        countdown = r.get("result", {}).get("countdown_timer", 0)
        print(f"    → 倒计时剩余: {countdown:.2f}s")

        # 等待倒计时结束
        section(7, "等待倒计时结束 → 自动进入 playing")
        wait_time = max(countdown + 0.3, 0.5)
        print(f"    等待 {wait_time:.1f}s ...")
        await asyncio.sleep(wait_time)
        r = await rpc(ws, "game.inspect")
        state = r.get("result", {}).get("state", "")
        print(f"    → 当前状态: {state}")

        # ━━━━━━━━ 阶段四：自动游玩 ━━━━━━━━
        section(8, f"自动游玩 — 操控小鸟通过 {TARGET_SCORE} 个管道")
        final_score, exit_state = await autopilot(ws)

        # ━━━━━━━━ 阶段五：自然死亡 ━━━━━━━━
        section(9, "等待小鸟自然死亡")
        if exit_state == "playing":
            print("    小鸟不再受控，等待重力和碰撞...")
            death_result = await wait_for_death(ws)
            if death_result:
                result = death_result["result"]
                print(f"    → 小鸟已死亡！")
                print(f"    → 最终分数: {result['score']}")
                print(f"    → 最高分: {result['best_score']}")
                print(f"    → 小鸟位置: y={result['bird']['y']:.1f}")
            else:
                print("    → 超时，小鸟仍未死亡")
        else:
            print(f"    小鸟已在控制循环中死亡 (state={exit_state})")

        section(10, "game.inspect — 验证最终死亡状态")
        await rpc(ws, "game.inspect")

        # ━━━━━━━━ 阶段六：远程修改验证 ━━━━━━━━
        section(11, "game.setBirdY — 远程修改小鸟 Y 坐标")
        await rpc(ws, "game.setBirdY", {"y": 100.0})

        section(12, "game.setScore — 远程修改分数")
        await rpc(ws, "game.setScore", {"score": 99})

        section(13, "game.inspect — 验证远程修改结果")
        await rpc(ws, "game.inspect")

        # ━━━━━━━━ 阶段七：截图 ━━━━━━━━
        section(14, "game.screenshot — VDP 远程截图")
        await rpc(ws, "game.screenshot", {"path": "/tmp/vdp_milestone_screenshot.png"})
        await asyncio.sleep(0.5)

        # ━━━━━━━━ 阶段八：错误处理 ━━━━━━━━
        section(15, "错误处理 — 调用不存在的方法")
        await rpc(ws, "game.nonexistent", {"foo": "bar"})

    print("\n" + "=" * 60)
    print("VDP 全流程验证完成")
    print("=" * 60)

asyncio.run(main())
