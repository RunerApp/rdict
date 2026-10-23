#!/usr/bin/env python3
"""Generate the rdict macOS app icon: five book spines on white
background, spells "rdict" — one letter per book. Gold-edged book
aesthetic. FINAL scheme: warm_lively (orange border) — user's pick from
the gen_icon_variants.py proposals (2026-09).
"""

from PIL import Image, ImageDraw, ImageFont, ImageFilter, ImageChops
import os
import subprocess

SIZE = 1024
CONTENT = int(SIZE * 0.80)
PAD = (SIZE - CONTENT) // 2

# ── Rounded rectangle mask (matches macOS standard) ──────────────────


def rounded_rect_mask(size: int) -> Image.Image:
    # Apple uses ~22.37% corner radius
    r = int(size * 0.2237)
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle([(0, 0), (size - 1, size - 1)], radius=r, fill=255)
    return mask


mask = rounded_rect_mask(CONTENT)

# ── Pure white background ────────────────────────────────────────────

bg = Image.new("RGBA", (CONTENT, CONTENT), (255, 255, 255, 255))
bg.putalpha(mask)

# ── Five book spines ─────────────────────────────────────────────────

canvas = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))

letters = list("rdict")
# Book colors — warm_lively scheme (user's pick): red / orange / yellow /
# green / teal books, orange border
colors = [
    (239, 68, 68),  # r — red    #EF4444
    (249, 115, 22),  # d — orange #F97316
    (250, 204, 21),  # i — yellow #FACC15
    (34, 197, 94),  # c — green  #22C55E
    (15, 157, 148),  # t — teal   #0F9D94
]
BORDER_COLOR = (249, 115, 22)  # orange #F97316

# Heights vary to create a skyline effect (smaller)
heights = [0.50, 0.60, 0.45, 0.66, 0.55]

num_books = 5
book_w = int(CONTENT * 0.14)
gap = 0  # no gap
total_w = num_books * book_w
start_x = (CONTENT - total_w) // 2
bottom_y = int(CONTENT * 0.82)


def get_font(size: int) -> ImageFont.FreeTypeFont:
    for p in [
        "/System/Library/Fonts/SF-Pro.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/Library/Fonts/SF-Pro.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]:
        try:
            return ImageFont.truetype(p, size)
        except:
            continue
    return ImageFont.load_default()


FONT_SIZE = int(CONTENT * 0.16)
font = get_font(FONT_SIZE)

# Shared absolute Y for letters and bottom gold band
min_book_h = int(CONTENT * min(heights))
# Shared BASELINE for all letters: with anchor="ms" every glyph sits on
# one typeset baseline — this is the alignment the user approved in the
# warm_lively variant render. (An ink-height equalization variant was
# tried and rejected: r/c read too large and the baseline math bugged
# out, letters sank to the book bottoms.)
letter_baseline_y = bottom_y - int(min_book_h * 0.45) + int(FONT_SIZE * 0.26)
gold_bottom_y = bottom_y - int(CONTENT * 0.06)

draw = ImageDraw.Draw(canvas)

for i, (letter, color, h_frac) in enumerate(zip(letters, colors, heights)):
    x = start_x + i * (book_w + gap)
    book_h = int(CONTENT * h_frac)
    top_y = bottom_y - book_h

    # Shadow under book (on the ground)
    shadow = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    sd.rounded_rectangle(
        [(x + 2, bottom_y + 2), (x + book_w + 2, bottom_y + 8)],
        radius=3,
        fill=(0, 0, 0, 50),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(6))
    canvas = Image.alpha_composite(canvas, shadow)

    # Book spine with leather texture
    spine = Image.new("RGBA", (book_w, book_h), (0, 0, 0, 0))
    sp_px = spine.load()

    for yy in range(book_h):
        for xx in range(book_w):
            # Vertical gradient: slightly lighter at top, darker at bottom
            vert = 0.80 + 0.20 * (1 - yy / book_h)
            # Horizontal gradient: subtle highlight in center (leather sheen)
            horiz = 0.92 + 0.08 * (1 - abs(xx - book_w / 2) / (book_w / 2))
            factor = vert * horiz
            r = min(255, int(color[0] * factor))
            g = min(255, int(color[1] * factor))
            b = min(255, int(color[2] * factor))
            sp_px[xx, yy] = (r, g, b, 255)

    # Add subtle noise for leather texture
    import random

    random.seed(42 + i)
    noise = Image.new("RGBA", (book_w, book_h), (0, 0, 0, 0))
    npx = noise.load()
    for yy in range(book_h):
        for xx in range(book_w):
            n = random.randint(-12, 12)
            r, g, b, a = sp_px[xx, yy]
            npx[xx, yy] = (
                max(0, min(255, r + n)),
                max(0, min(255, g + n)),
                max(0, min(255, b + n)),
                255,
            )
    spine = noise

    band = ImageDraw.Draw(spine)

    # Gold bands: bottom at fixed absolute Y (all aligned),
    # top mirrors bottom around each book's own center (symmetric)
    bot_band_abs = gold_bottom_y  # shared across all books
    book_center_y = top_y + book_h // 2
    # Top band = mirror of bottom band about book center
    top_band_abs = 2 * book_center_y - bot_band_abs
    top_band_rel = top_band_abs - top_y
    bot_band_rel = bot_band_abs - top_y

    gold = (220, 185, 100, 230)
    gold_dark = (180, 145, 70, 230)

    if 0 <= top_band_rel < book_h:
        band.rectangle([(0, top_band_rel), (book_w - 1, top_band_rel + 3)], fill=gold)
        band.rectangle(
            [(0, top_band_rel + 5), (book_w - 1, top_band_rel + 6)], fill=gold_dark
        )
    if 0 <= bot_band_rel < book_h:
        band.rectangle([(0, bot_band_rel), (book_w - 1, bot_band_rel + 3)], fill=gold)
        band.rectangle(
            [(0, bot_band_rel + 5), (book_w - 1, bot_band_rel + 6)], fill=gold_dark
        )

    # Letter: shared baseline ("ms"), same font size for every glyph —
    # reads as typeset "rdict" across the spines
    letter_y_rel = letter_baseline_y - top_y
    # Text shadow for embossed effect
    band.text(
        (book_w // 2 + 1, letter_y_rel + 1),
        letter,
        font=font,
        fill=(0, 0, 0, 100),
        anchor="ms",
    )
    band.text(
        (book_w // 2, letter_y_rel),
        letter,
        font=font,
        fill=(255, 255, 255, 245),
        anchor="ms",
    )

    # Round only top corners (books stand on a surface)
    spine_mask = Image.new("L", (book_w, book_h), 0)
    sm = ImageDraw.Draw(spine_mask)
    sm.rounded_rectangle([(0, 0), (book_w - 1, book_h - 1)], radius=4, fill=255)
    spine.putalpha(spine_mask)

    canvas.paste(spine, (x, top_y), spine)

# ── Composite: bg + books ────────────────────────────────────────────

icon = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
icon.paste(bg, (0, 0), bg)
icon = Image.alpha_composite(icon, canvas)

# ── Black border: draw ON TOP, clipped to mask ───────────────────────

border = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
bd = ImageDraw.Draw(border)
r = int(CONTENT * 0.2237)
# Draw border at the exact mask edge so no white shows through
bd.rounded_rectangle(
    [(0, 0), (CONTENT - 1, CONTENT - 1)],
    radius=r,
    outline=(*BORDER_COLOR, 255),
    width=28,
)
# Multiply border alpha by mask so it only shows within the rounded rect
border_alpha = border.split()[3]
clipped_alpha = ImageChops.multiply(border_alpha, mask)
border.putalpha(clipped_alpha)
icon = Image.alpha_composite(icon, border)

# ── Final ────────────────────────────────────────────────────────────

final = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
final.paste(icon, (PAD, PAD), icon)

# Also save a raw preview to /tmp
final.save("/tmp/rdict_icon_1024.png", "PNG")
print("Preview: /tmp/rdict_icon_1024.png")

# ── Save ─────────────────────────────────────────────────────────────

out_dir = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..",
    "apps",
    "apple",
    "RdictApp.app",
    "Contents",
    "Resources",
)
out_dir = os.path.abspath(out_dir)
os.makedirs(out_dir, exist_ok=True)

png_path = os.path.join(out_dir, "AppIcon.png")
final.save(png_path, "PNG")

iconset_dir = os.path.join(out_dir, "AppIcon.iconset")
os.makedirs(iconset_dir, exist_ok=True)
for s in [16, 32, 64, 128, 256, 512, 1024]:
    final.resize((s, s), Image.LANCZOS).save(
        os.path.join(iconset_dir, f"icon_{s}x{s}.png"), "PNG"
    )
    if s <= 512:
        final.resize((s * 2, s * 2), Image.LANCZOS).save(
            os.path.join(iconset_dir, f"icon_{s}x{s}@2x.png"), "PNG"
        )

icns_path = os.path.join(out_dir, "AppIcon.icns")
try:
    subprocess.run(
        ["iconutil", "-c", "icns", iconset_dir, "-o", icns_path],
        check=True,
        capture_output=True,
    )
    os.system(f"rm -rf '{iconset_dir}'")
except Exception as e:
    print(f"iconutil: {e}")

# Re-sign the app: replacing resources invalidates the code signature,
# and macOS may fall back to cached icons for invalid-signature bundles.
app_bundle = os.path.join(out_dir, "..", "..", "..")
subprocess.run(
    ["codesign", "--force", "--deep", "--sign", "-", app_bundle],
    capture_output=True,
)

print(f"Saved: {png_path}")
