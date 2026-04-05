# Tetris — Vibe2D Example

A Guideline Tetris implementation built with the [Vibe2D](../../README.md) engine, demonstrating sprite rendering, input handling, SRS rotation system, and the Vibe Debug Protocol (VDP) for automated testing and AI play.

## Features

- **SRS (Super Rotation System)** with wall-kick tables for JLSTZ and I pieces
- **7-Bag randomization** for fair piece distribution
- **Guideline scoring** with T-Spin detection, combos, and back-to-back bonuses
- **Hold piece** and **5-piece next queue** preview
- **Ghost piece** showing landing position
- **VDP integration** with full game state inspection and custom control methods
- **VDP test suite** (`tests/vdp_full_test.py`) — 23 automated tests covering all game mechanics
- **AI player** (`tests/autopilot_max_score.py`) — Pierre Dellacherie evaluation with optional two-piece lookahead

## Acknowledgements

This example was inspired by and borrows assets and ideas from [tetris-love](https://github.com/nununoisy/tetris-love) by [@nununoisy](https://github.com/nununoisy). Background image, block textures, and font are from the original project. Thank you for the great reference! 🙏
