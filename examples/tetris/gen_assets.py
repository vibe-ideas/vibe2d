#!/usr/bin/env python3
"""Generate tiny colored PNG block assets for Tetris."""
import struct
import zlib
import os

def create_png(width, height, r, g, b, a=255):
    """Create a minimal PNG file in memory (no dependencies)."""
    def chunk(chunk_type, data):
        c = chunk_type + data
        crc = struct.pack('>I', zlib.crc32(c) & 0xffffffff)
        return struct.pack('>I', len(data)) + c + crc

    header = b'\x89PNG\r\n\x1a\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))

    raw = b''
    for _ in range(height):
        raw += b'\x00'  # filter byte
        for _ in range(width):
            raw += struct.pack('BBBB', r, g, b, a)

    idat = chunk(b'IDAT', zlib.compress(raw))
    iend = chunk(b'IEND', b'')

    return header + ihdr + idat + iend

BLOCKS = {
    'block_i':      (0, 240, 240, 255),    # cyan
    'block_o':      (240, 240, 0, 255),     # yellow
    'block_t':      (160, 0, 240, 255),     # purple
    'block_s':      (0, 240, 0, 255),       # green
    'block_z':      (240, 0, 0, 255),       # red
    'block_j':      (0, 0, 240, 255),       # blue
    'block_l':      (240, 160, 0, 255),     # orange
    'block_ghost':  (255, 255, 255, 80),    # white semi-transparent
    'block_grid':   (26, 26, 46, 255),      # dark gray
    'block_border': (85, 85, 85, 255),      # medium gray
}

def main():
    out_dir = os.path.join(os.path.dirname(__file__), 'assets', 'blocks')
    os.makedirs(out_dir, exist_ok=True)

    for name, (r, g, b, a) in BLOCKS.items():
        path = os.path.join(out_dir, f'{name}.png')
        data = create_png(2, 2, r, g, b, a)
        with open(path, 'wb') as f:
            f.write(data)
        print(f'  created {path} ({len(data)} bytes)')

    print('Done!')

if __name__ == '__main__':
    main()
