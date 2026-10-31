#!/usr/bin/env python3
"""Generate FuseFlow launcher icons (pure stdlib, no PIL).

Design: rounded-square blue->emerald gradient, white flame (fusion) over a
white flow wave, small droplet beside it.

Usage:
  python3 scripts/gen_icon.py [size]      -> assets/icons/icon.png + sizes
  python3 scripts/gen_icon.py --android   -> inject icons into the generated
                                             Android project under target/dx
Writes: assets/icons/icon.png and assets/icons/icon_<size>.png
"""
import math
import os
import struct
import sys
import zlib

ANDROID_MODE = "--android" in sys.argv
args = [a for a in sys.argv[1:] if a != "--android"]
MAIN = int(args[0]) if args else 512
SS = 3  # supersample factor for anti-aliasing

TOP = (29, 78, 216)   # blue-800 #1d4ed8
MID = (13, 148, 136)  # teal-600
BOT = (16, 185, 129)  # emerald-500
WHITE = (255, 255, 255)


def clamp(x, lo=0.0, hi=1.0):
    return lo if x < lo else hi if x > hi else x


def cov(dist, w):
    """Coverage 0..1 for signed distance `dist` (negative = inside), aa width w."""
    return clamp(0.5 - dist / w)


def sd_round_rect(u, v, cx, cy, hw, hh, r):
    dx = abs(u - cx) - (hw - r)
    dy = abs(v - cy) - (hh - r)
    ox, oy = max(dx, 0.0), max(dy, 0.0)
    return math.hypot(ox, oy) + min(max(dx, dy), 0.0) - r


def sd_flame(u, v, cx, cy, w, h, tip=2.6):
    """Superellipse teardrop; exponent `tip` > 2 sharpens the top into a tip."""
    ny = min((v - cy) / h, 1.0)
    k = 2.0 + (tip - 2.0) * clamp(ny)
    nx = (u - cx) / w
    e = math.pow(abs(nx), k) + math.pow(abs(ny), k)
    return (e ** (1.0 / k) - 1.0) * min(w, h)


def lerp(a, b, t):
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))


def render(side):
    """Render a `side`x`side` RGBA image via SS^2 supersampling."""
    n = side * SS
    px = bytearray()
    aa = 1.6 / n  # aa width in uv units
    for j in range(n):
        v = 1.0 - (j + 0.5) / n  # v grows upward
        for i in range(n):
            u = (i + 0.5) / n

            d_bg = sd_round_rect(u, v, 0.5, 0.5, 0.5, 0.5, 0.22)
            a_bg = cov(d_bg, aa)
            if a_bg <= 0.0:
                px += bytes(4)
                continue

            bg = lerp(MID, TOP, (v - 0.5) * 2) if v > 0.5 else lerp(BOT, MID, v * 2)

            a_f = cov(sd_flame(u, v, 0.50, 0.52, 0.20, 0.26), aa)
            wave = 0.285 + 0.045 * math.sin((u - 0.5) * 3 * math.pi)
            a_w = cov(abs(v - wave) - 0.030, aa) * clamp((0.78 - abs(u - 0.5)) * 8)
            a_d = cov(sd_flame(u, v, 0.765, 0.55, 0.065, 0.09, tip=2.8), aa)

            col = bg
            for a in (a_w, a_d, a_f):
                if a > 0:
                    col = lerp(col, WHITE, a)

            r, g, b = (int(clamp(x / 255.0) * 255 + 0.5) for x in col)
            px += bytes((r, g, b, int(a_bg * 255 + 0.5)))
    return bytes(px), side


def write_png(path, data, width, height):
    def chunk(tag, body):
        c = tag + body
        return struct.pack(">I", len(body)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + data[y * width * 4:(y + 1) * width * 4] for y in range(height))
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    open(path, "wb").write(png)


if ANDROID_MODE:
    # Android launcher densities: name -> px
    densities = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}
    # find generated Android res dir
    base = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    res = os.path.join(base, "target", "dx", "private-dns-switcher", "debug", "android", "app",
                       "app", "src", "main", "res")
    if not os.path.isdir(res):
        sys.exit(f"android project not built yet: {res} (run `make build-android` first)")
    removed = 0
    for dpi, size in densities.items():
        d = os.path.join(res, f"mipmap-{dpi}")
        os.makedirs(d, exist_ok=True)
        img, _ = render(size)
        for old in ("ic_launcher.webp", "ic_launcher.png", "ic_launcher_round.webp",
                    "ic_launcher_round.png"):
            p = os.path.join(d, old)
            if os.path.exists(p):
                os.remove(p)
                removed += 1
        write_png(os.path.join(d, "ic_launcher.png"), img, size, size)
    # adaptive-icon xml (API 26+) would override PNGs; remove so our art shows
    anydpi = os.path.join(res, "mipmap-anydpi-v26")
    if os.path.isdir(anydpi):
        for f in os.listdir(anydpi):
            os.remove(os.path.join(anydpi, f))
        os.rmdir(anydpi)
        removed += 1
    print(f"ok: injected android mipmaps ({densities.values()}), removed {removed} template files")
else:
    img, _ = render(MAIN)
    write_png("assets/icons/icon.png", img, MAIN, MAIN)
    for s in (192, 96, 48):
        small, _ = render(s)
        write_png(f"assets/icons/icon_{s}.png", small, s, s)
    print(f"ok: assets/icons/icon.png ({MAIN}x{MAIN}) + icon_192/96/48.png")
