#!/usr/bin/env python3
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw, ImageFilter
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[1]
ASSET_DIR = ROOT / "Resources" / "icons"
ELECTRON_DIR = ROOT / "electron" / "assets"
ICONSET_DIR = ASSET_DIR / "Keydock.iconset"


def rounded_rect_mask(size, radius):
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    return mask


def generate_icon(size=1024):
    scale = size / 1024
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    blob_mask = Image.new("L", (size, size), 0)
    mask_draw = ImageDraw.Draw(blob_mask)
    circles = [
        (146, 272, 402),
        (314, 158, 444),
        (560, 190, 402),
        (660, 336, 300),
        (606, 532, 302),
        (372, 596, 330),
        (178, 500, 284),
    ]
    for x, y, diameter in circles:
        mask_draw.ellipse(
            (
                int(x * scale),
                int(y * scale),
                int((x + diameter) * scale),
                int((y + diameter) * scale),
            ),
            fill=255,
        )
    mask_draw.rounded_rectangle(
        (
            int(188 * scale),
            int(294 * scale),
            int(838 * scale),
            int(768 * scale),
        ),
        radius=int(132 * scale),
        fill=255,
    )
    blob_mask = blob_mask.filter(ImageFilter.GaussianBlur(int(2 * scale)))

    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    shadow.paste(Image.new("RGBA", (size, size), (35, 49, 122, 108)), (0, int(26 * scale)), blob_mask)
    shadow = shadow.filter(ImageFilter.GaussianBlur(int(34 * scale)))
    image.alpha_composite(shadow)

    fill = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    pixels = fill.load()
    for y in range(size):
        vertical = y / max(1, size - 1)
        for x in range(size):
            horizontal = x / max(1, size - 1)
            magenta = max(0, 1 - ((x - size * 0.45) ** 2 + (y - size * 0.20) ** 2) / (size * size * 0.34))
            cyan = max(0, 1 - ((x - size * 0.17) ** 2 + (y - size * 0.48) ** 2) / (size * size * 0.20))
            deep = max(0, 1 - ((x - size * 0.48) ** 2 + (y - size * 0.82) ** 2) / (size * size * 0.16))
            r = int(38 + 56 * (1 - vertical) + 52 * magenta + 18 * cyan)
            g = int(86 + 40 * (1 - vertical) + 14 * magenta + 58 * cyan - 24 * deep)
            b = int(238 + 12 * (1 - horizontal) + 10 * magenta - 18 * deep)
            pixels[x, y] = (min(255, r), min(255, g), min(255, b), 255)
    fill.putalpha(blob_mask)
    image.alpha_composite(fill)

    rim = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    rim_mask = blob_mask.filter(ImageFilter.MaxFilter(max(3, int(11 * scale) | 1)))
    rim_alpha = Image.composite(rim_mask, Image.new("L", (size, size), 0), rim_mask)
    rim_alpha = ImageChops.subtract(rim_alpha, blob_mask.filter(ImageFilter.MinFilter(max(3, int(5 * scale) | 1))))
    rim.putalpha(rim_alpha)
    rim_draw = ImageDraw.Draw(rim)
    rim_draw.bitmap((0, 0), rim_alpha, fill=(145, 177, 255, 120))
    image.alpha_composite(rim)

    gloss = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    gloss_draw = ImageDraw.Draw(gloss)
    gloss_draw.ellipse(
        (
            int(236 * scale),
            int(172 * scale),
            int(758 * scale),
            int(518 * scale),
        ),
        fill=(255, 255, 255, 46),
    )
    gloss.putalpha(Image.composite(gloss.getchannel("A"), Image.new("L", (size, size), 0), blob_mask))
    gloss = gloss.filter(ImageFilter.GaussianBlur(int(8 * scale)))
    image.alpha_composite(gloss)

    symbol_shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    symbol_draw = ImageDraw.Draw(symbol_shadow)
    white = (255, 255, 255, 238)
    sw = max(6, int(56 * scale))
    symbol_draw.line(
        (
            int(340 * scale),
            int(410 * scale),
            int(438 * scale),
            int(512 * scale),
            int(340 * scale),
            int(614 * scale),
        ),
        fill=white,
        width=sw,
        joint="curve",
    )
    symbol_draw.line(
        (
            int(514 * scale),
            int(585 * scale),
            int(650 * scale),
            int(585 * scale),
        ),
        fill=white,
        width=sw,
    )
    symbol_shadow = symbol_shadow.filter(ImageFilter.GaussianBlur(int(12 * scale)))
    symbol_shadow.putalpha(Image.composite(symbol_shadow.getchannel("A"), Image.new("L", (size, size), 0), blob_mask))
    image.alpha_composite(symbol_shadow)

    draw = ImageDraw.Draw(image)
    draw.line(
        (
            int(340 * scale),
            int(410 * scale),
            int(438 * scale),
            int(512 * scale),
            int(340 * scale),
            int(614 * scale),
        ),
        fill=(255, 255, 255, 255),
        width=sw,
        joint="curve",
    )
    draw.line(
        (
            int(514 * scale),
            int(585 * scale),
            int(650 * scale),
            int(585 * scale),
        ),
        fill=(255, 255, 255, 255),
        width=sw,
    )

    key_color = (255, 255, 255, 252)
    key_shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    kd = ImageDraw.Draw(key_shadow)
    kd.ellipse(
        (
            int(624 * scale),
            int(354 * scale),
            int(756 * scale),
            int(486 * scale),
        ),
        outline=key_color,
        width=max(6, int(36 * scale)),
    )
    kd.line(
        (
            int(700 * scale),
            int(470 * scale),
            int(792 * scale),
            int(562 * scale),
        ),
        fill=key_color,
        width=max(6, int(38 * scale)),
    )
    kd.line(
        (
            int(758 * scale),
            int(526 * scale),
            int(806 * scale),
            int(478 * scale),
        ),
        fill=key_color,
        width=max(6, int(32 * scale)),
    )
    kd.line(
        (
            int(784 * scale),
            int(552 * scale),
            int(828 * scale),
            int(508 * scale),
        ),
        fill=key_color,
        width=max(6, int(32 * scale)),
    )
    key_shadow = key_shadow.filter(ImageFilter.GaussianBlur(int(8 * scale)))
    image.alpha_composite(key_shadow)
    draw = ImageDraw.Draw(image)
    draw.ellipse(
        (
            int(624 * scale),
            int(354 * scale),
            int(756 * scale),
            int(486 * scale),
        ),
        outline=key_color,
        width=max(6, int(36 * scale)),
    )
    draw.line(
        (
            int(700 * scale),
            int(470 * scale),
            int(792 * scale),
            int(562 * scale),
        ),
        fill=key_color,
        width=max(6, int(38 * scale)),
    )
    draw.line(
        (
            int(758 * scale),
            int(526 * scale),
            int(806 * scale),
            int(478 * scale),
        ),
        fill=key_color,
        width=max(6, int(32 * scale)),
    )
    draw.line(
        (
            int(784 * scale),
            int(552 * scale),
            int(828 * scale),
            int(508 * scale),
        ),
        fill=key_color,
        width=max(6, int(32 * scale)),
    )

    return image


def save_pngs(source):
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    ELECTRON_DIR.mkdir(parents=True, exist_ok=True)
    source.save(ASSET_DIR / "keydock-icon.png")
    source.save(ELECTRON_DIR / "icon.png")

    sizes = [16, 32, 64, 128, 256, 512, 1024]
    for size in sizes:
        source.resize((size, size), Image.Resampling.LANCZOS).save(ASSET_DIR / f"keydock-icon-{size}.png")

    ico_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    source.save(ELECTRON_DIR / "icon.ico", sizes=ico_sizes)


def save_svg():
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    svg = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="cloud" x1="0.18" y1="0.1" x2="0.82" y2="0.95">
      <stop offset="0" stop-color="#b9a3ff"/>
      <stop offset="0.42" stop-color="#4e8dff"/>
      <stop offset="1" stop-color="#1e2cff"/>
    </linearGradient>
    <filter id="softShadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="30" stdDeviation="26" flood-color="#22317a" flood-opacity=".28"/>
    </filter>
    <filter id="glow" x="-35%" y="-35%" width="170%" height="170%">
      <feGaussianBlur stdDeviation="9"/>
    </filter>
  </defs>
  <path d="M151 473c-44-109 22-217 135-233 36-100 147-148 253-96 70-42 181-23 236 49 92 10 155 89 142 179 69 55 77 157 19 222 16 98-64 180-167 169-61 91-194 111-278 40-93 35-204-2-246-92-99-14-160-126-94-238z" fill="url(#cloud)" filter="url(#softShadow)"/>
  <path d="M236 263c134-49 334-21 485 101" fill="none" stroke="#fff" stroke-opacity=".28" stroke-width="28" stroke-linecap="round"/>
  <g fill="none" stroke="#fff" stroke-linecap="round" stroke-linejoin="round" filter="url(#glow)" opacity=".72">
    <path d="M340 410l98 102-98 102" stroke-width="58"/>
    <path d="M514 585h136" stroke-width="58"/>
    <circle cx="690" cy="420" r="50" stroke-width="36"/>
    <path d="M700 470l92 92M758 526l48-48M784 552l44-44" stroke-width="38"/>
  </g>
  <g fill="none" stroke="#fff" stroke-linecap="round" stroke-linejoin="round">
    <path d="M340 410l98 102-98 102" stroke-width="56"/>
    <path d="M514 585h136" stroke-width="56"/>
    <circle cx="690" cy="420" r="50" stroke-width="36"/>
    <path d="M700 470l92 92M758 526l48-48M784 552l44-44" stroke-width="38"/>
  </g>
</svg>
"""
    (ASSET_DIR / "keydock-icon.svg").write_text(svg, encoding="utf-8")


def save_icns(source):
    if ICONSET_DIR.exists():
        shutil.rmtree(ICONSET_DIR)
    ICONSET_DIR.mkdir(parents=True)
    specs = [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]
    for filename, size in specs:
        source.resize((size, size), Image.Resampling.LANCZOS).save(ICONSET_DIR / filename)
    iconutil = shutil.which("iconutil")
    if iconutil:
        result = subprocess.run(
            [iconutil, "-c", "icns", str(ICONSET_DIR), "-o", str(ASSET_DIR / "Keydock.icns")],
            check=False,
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            shutil.copyfile(ASSET_DIR / "Keydock.icns", ELECTRON_DIR / "icon.icns")
            return
    write_icns_fallback(source, ASSET_DIR / "Keydock.icns")
    shutil.copyfile(ASSET_DIR / "Keydock.icns", ELECTRON_DIR / "icon.icns")


def write_icns_fallback(source, out_path):
    # Modern icns files can store PNG payloads in size-specific chunks.
    chunk_specs = [
        ("icp4", 16),
        ("icp5", 32),
        ("icp6", 64),
        ("ic07", 128),
        ("ic08", 256),
        ("ic09", 512),
        ("ic10", 1024),
    ]
    chunks = []
    for code, size in chunk_specs:
        png_path = ASSET_DIR / f".icns-{size}.png"
        source.resize((size, size), Image.Resampling.LANCZOS).save(png_path)
        payload = png_path.read_bytes()
        png_path.unlink()
        chunks.append(code.encode("ascii") + (len(payload) + 8).to_bytes(4, "big") + payload)
    data = b"".join(chunks)
    out_path.write_bytes(b"icns" + (len(data) + 8).to_bytes(4, "big") + data)


def main():
    icon = generate_icon(1024)
    save_pngs(icon)
    save_svg()
    save_icns(icon)


if __name__ == "__main__":
    main()
