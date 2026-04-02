# Vibe2D VDP Skill

This skill enables AI assistants to inspect and control a running Vibe2D game via the Vibe Debug Protocol (VDP).

## Prerequisites

- A Vibe2D game must be running with VDP enabled in `game.yaml`:
  ```yaml
  debug:
    vdp:
      enabled: true
      port: 9229
  ```

## Available Commands

### Inspect Game State
```bash
vibe inspect
```
Returns the full game state as JSON, including:
- Game state machine state (idle, countdown, playing, dead)
- Score and best score
- Entity positions (bird, pipes, etc.)

### Send Custom RPC
```bash
vibe rpc <method> [params_json]
```
Examples:
```bash
# Get engine info
vibe rpc engine.info

# Get game state
vibe rpc game.inspect

# Set bird position
vibe rpc game.setBirdY '{"y": 100}'

# Set score
vibe rpc game.setScore '{"score": 42}'

# Change game state
vibe rpc game.setState '{"state": "playing"}'
```

### Create New Project
```bash
vibe new my-game
```

## VDP Protocol Reference

VDP uses WebSocket + JSON-RPC 2.0 on `ws://127.0.0.1:9229`.

### Request Format
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "game.inspect",
  "params": {}
}
```

### Response Format
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { ... }
}
```

### Built-in Methods

| Method | Description |
|--------|-------------|
| `engine.info` | Engine version, virtual resolution |
| `game.inspect` | Full game state JSON |

### Game-specific Methods (Flappy Bird)

| Method | Params | Description |
|--------|--------|-------------|
| `game.setBirdY` | `{"y": float}` | Set bird Y position |
| `game.setScore` | `{"score": int}` | Set current score |
| `game.setState` | `{"state": string}` | Set game state (idle/countdown/playing/dead) |

## Implementing VDP in Your Game

Override `inspect()` and `handle_vdp()` in your Game trait:

```rust
fn inspect(&self) -> serde_json::Value {
    serde_json::json!({
        "state": "playing",
        "score": self.score,
        "player": { "x": self.x, "y": self.y },
    })
}

fn handle_vdp(&mut self, method: &str, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    match method {
        "game.setPlayerPos" => {
            self.x = params["x"].as_f64().unwrap() as f32;
            self.y = params["y"].as_f64().unwrap() as f32;
            Ok(serde_json::json!({"ok": true}))
        }
        _ => Err(format!("Unknown: {}", method)),
    }
}
```
