#!/usr/bin/env python3
"""Generate the Enclave app icons (desktop + Android) from the brand mark.

The mark — "the keyhole vault" — is a violet→indigo diagonal gradient tile
with a white keyhole cut out. This script must stay in sync with
packages/ui/src/components/Logo.svelte (the in-app SVG version).

Usage: python3 scripts/generate-icons.py   (needs Pillow)
Outputs:
  src-tauri/icons/            desktop PNGs + multi-size .ico
  src-tauri/gen/android/...   legacy launcher + adaptive-icon foreground PNGs
Also writes logo-preview.png (1024px) at the repo root for eyeballing.
"""

import os
import sys

from PIL import Image, ImageChops, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
S = 2048  # supersampled draw size; everything is downscaled from here

VIOLET = (138, 123, 255)   # #8a7bff
INDIGO = (79, 70, 229)     # #4f46e5
WHITE = (255, 255, 255)


def diagonal_gradient(w: int, h: int, c1, c2) -> Image.Image:
    """Exact diagonal gradient c1 (top-left) → c2 (bottom-right)."""
    vert = Image.linear_gradient("L").resize((w, h))
    horiz = vert.rotate(90)
    t = ImageChops.add(vert, horiz).point(lambda v: v // 2)  # 0..255 by (x+y)
    r = t.point(lambda v: round(c1[0] + (c2[0] - c1[0]) * v / 255))
    g = t.point(lambda v: round(c1[1] + (c2[1] - c1[1]) * v / 255))
    b = t.point(lambda v: round(c1[2] + (c2[2] - c1[2]) * v / 255))
    return Image.merge("RGB", (r, g, b))


def keyhole_masks(size: int):
    """(body, hole) alpha masks for the keyhole, scaled from the 48-unit SVG."""
    k = size / 48
    body = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(body)
    d.ellipse((24 * k - 11 * k, 8 * k, 24 * k + 11 * k, 30 * k), fill=255)
    d.rounded_rectangle((16.6 * k, 27.3 * k, 31.4 * k, 36 * k), radius=3 * k, fill=255)
    hole = Image.new("L", (size, size), 0)
    ImageDraw.Draw(hole).ellipse(
        (24 * k - 4.2 * k, 17.5 * k - 4.2 * k, 24 * k + 4.2 * k, 17.5 * k + 4.2 * k),
        fill=255,
    )
    return body, hole


def logo(size: int) -> Image.Image:
    """The full brand mark at `size` px: gradient tile + white keyhole."""
    grad = diagonal_gradient(S, S, VIOLET, INDIGO)
    tile_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(tile_mask).rounded_rectangle((0, 0, S - 1, S - 1), radius=S * 0.25, fill=255)

    body, hole = keyhole_masks(S)
    white = Image.new("RGB", (S, S), WHITE)
    keyhole = Image.composite(white, grad, body)              # white keyhole on gradient
    out = Image.composite(grad, keyhole, hole)                # hole shows the gradient again
    out.putalpha(tile_mask)
    return out.resize((size, size), Image.LANCZOS)


def adaptive_foreground(size: int) -> Image.Image:
    """Adaptive-icon foreground: the mark at 66/108 of the canvas, centered."""
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    art = logo(round(size * 66 / 108))
    canvas.paste(art, ((size - art.width) // 2, (size - art.height) // 2), art)
    return canvas


# ── Alternate logo concepts (previews only, `--concepts` flag) ──
# Concept 1 (the shipped mark) is above; these are alternatives to compare.

def _font(size_px: int):
    for path in (
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    ):
        if os.path.exists(path):
            from PIL import ImageFont

            return ImageFont.truetype(path, size_px)
    return None


def concept_shield_e(size: int) -> Image.Image:
    """Concept 2: white shield with an E cut out (heraldic)."""
    grad = diagonal_gradient(S, S, VIOLET, INDIGO)
    tile_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(tile_mask).rounded_rectangle((0, 0, S - 1, S - 1), radius=S * 0.25, fill=255)
    # Shield polygon: rounded top corners via a rounded rect + triangle bottom.
    shield = Image.new("L", (S, S), 0)
    ds = ImageDraw.Draw(shield)
    ds.rounded_rectangle((S * 0.12, S * 0.1, S * 0.88, S * 0.62), radius=S * 0.12, fill=255)
    ds.polygon([(S * 0.12, S * 0.55), (S * 0.88, S * 0.55), (S * 0.5, S * 0.95)], fill=255)
    white = Image.new("RGB", (S, S), WHITE)
    out = Image.composite(white, grad, shield)
    # E cutout — reuse the gradient as the letter fill.
    fnt = _font(S * 0.34)
    if fnt:
        tmp = Image.new("L", (S, S), 0)
        ImageDraw.Draw(tmp).text((S * 0.5, S * 0.47), "E", font=fnt, fill=255, anchor="mm")
        out = Image.composite(grad, out, tmp)
    out.putalpha(tile_mask)
    return out.resize((size, size), Image.LANCZOS)


def concept_hexagon(size: int) -> Image.Image:
    """Concept 3: hexagon fortress (an enclave is an enclosed territory)."""
    grad = diagonal_gradient(S, S, VIOLET, INDIGO)
    body, hole = keyhole_masks(S)
    white = Image.new("RGB", (S, S), WHITE)
    hex_mask = Image.new("L", (S, S), 0)
    h = S * 0.433
    cx = S / 2
    ImageDraw.Draw(hex_mask).polygon(
        [(cx, S * 0.04), (S * 0.92, S * 0.29), (S * 0.92, S * 0.71), (cx, S * 0.96), (S * 0.08, S * 0.71), (S * 0.08, S * 0.29)],
        fill=255,
    )
    keyhole = Image.composite(white, grad, body)
    out = Image.composite(grad, keyhole, hole)
    out.putalpha(hex_mask)
    return out.resize((size, size), Image.LANCZOS)


def concept_layers(size: int) -> Image.Image:
    """Concept 4: layered vault — data inside the vault."""
    grad = diagonal_gradient(S, S, VIOLET, INDIGO)
    tile_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(tile_mask).rounded_rectangle((0, 0, S - 1, S - 1), radius=S * 0.25, fill=255)
    # Back page (white, offset up-left) + front page (gradient, keyhole).
    back = Image.new("RGB", (S, S), WHITE)
    back_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(back_mask).rounded_rectangle((S * 0.08, S * 0.16, S * 0.88, S * 0.84), radius=S * 0.07, fill=255)
    front_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(front_mask).rounded_rectangle((S * 0.18, S * 0.1, S * 0.94, S * 0.82), radius=S * 0.07, fill=255)
    body, hole = keyhole_masks(S)
    # Small keyhole on the front page.
    k_small = Image.new("L", (S, S), 0)
    ImageDraw.Draw(k_small).ellipse((S * 0.42, S * 0.32, S * 0.62, S * 0.52), fill=255)
    white = Image.new("RGB", (S, S), WHITE)
    out = Image.composite(white, grad, back_mask)
    front = Image.composite(grad, white, front_mask)
    out = Image.composite(front, out, front_mask)
    out = Image.composite(white, out, k_small)
    out.putalpha(tile_mask)
    return out.resize((size, size), Image.LANCZOS)


CONCEPTS = {
    "logo-concept-2-shield-e.png": concept_shield_e,
    "logo-concept-3-hexagon.png": concept_hexagon,
    "logo-concept-4-layers.png": concept_layers,
}


def write_concepts() -> None:
    for name, fn in CONCEPTS.items():
        fn(1024).save(os.path.join(ROOT, name))
    print("concept previews written to " + ", ".join(CONCEPTS))


def main() -> None:
    icon_dir = os.path.join(ROOT, "src-tauri", "icons")
    android_res = os.path.join(ROOT, "src-tauri", "gen", "android", "app", "src", "main", "res")

    # Desktop icons
    for name, px in [
        ("32x32.png", 32),
        ("64x64.png", 64),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
    ]:
        logo(px).save(os.path.join(icon_dir, name))
    # Windows Store tiles
    for name, px in [
        ("StoreLogo.png", 50),
        ("Square30x30Logo.png", 30),
        ("Square44x44Logo.png", 44),
        ("Square71x71Logo.png", 71),
        ("Square89x89Logo.png", 89),
        ("Square107x107Logo.png", 107),
        ("Square142x142Logo.png", 142),
        ("Square150x150Logo.png", 150),
        ("Square284x284Logo.png", 284),
        ("Square310x310Logo.png", 310),
    ]:
        logo(px).save(os.path.join(icon_dir, name))
    # Multi-size .ico
    logo(256).save(
        os.path.join(icon_dir, "icon.ico"),
        format="ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )

    # Android: legacy + round launcher icons and adaptive foreground per density
    for density, px in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)]:
        res = os.path.join(android_res, f"mipmap-{density}")
        logo(px).save(os.path.join(res, "ic_launcher.png"))
        logo(px).save(os.path.join(res, "ic_launcher_round.png"))
        adaptive_foreground(px).save(os.path.join(res, "ic_launcher_foreground.png"))

    # Adaptive-icon background color (was the default Android green)
    bg_xml = os.path.join(android_res, "drawable", "ic_launcher_background.xml")
    with open(bg_xml, "r") as f:
        xml = f.read()
    xml = xml.replace('#3DDC84', '#4F46E5').replace('#3ddc84', '#4F46E5')
    with open(bg_xml, "w") as f:
        f.write(xml)

    logo(1024).save(os.path.join(ROOT, "logo-preview.png"))
    print(f"icons written to {icon_dir} and {android_res}; preview: {ROOT}/logo-preview.png")


if __name__ == "__main__":
    try:
        if len(sys.argv) > 1 and sys.argv[1] == "--concepts":
            write_concepts()
        else:
            main()
    except ImportError:
        print("Pillow is required: pip install Pillow", file=sys.stderr)
        sys.exit(1)
