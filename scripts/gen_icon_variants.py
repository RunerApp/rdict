#!/usr/bin/env python3
"""Generate color variants of the rdict book icon with user-specified schemes."""

from PIL import Image, ImageDraw, ImageFont, ImageFilter, ImageChops
import random

SIZE = 1024
CONTENT = int(SIZE * 0.80)
PAD = (SIZE - CONTENT) // 2


def rounded_rect_mask(size):
    r = int(size * 0.2237)
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle([(0, 0), (size - 1, size - 1)], radius=r, fill=255)
    return mask


mask = rounded_rect_mask(CONTENT)


def get_font(size):
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

letters = list("rdict")
heights = [0.50, 0.60, 0.45, 0.66, 0.55]

book_w = int(CONTENT * 0.14)
total_w = 5 * book_w
start_x = (CONTENT - total_w) // 2
bottom_y = int(CONTENT * 0.82)
min_book_h = int(CONTENT * min(heights))
# Shared BASELINE (see render_icon): x-height ≈ 0.52 em → baseline sits
# ~0.26 em below the x-height center.
letter_baseline_y = bottom_y - int(min_book_h * 0.45) + int(FONT_SIZE * 0.26)
gold_bottom_y = bottom_y - int(CONTENT * 0.06)

# Pre-compute each letter's ink bbox for visual alignment
letter_ink = {}
for ch in letters:
    bbox = font.getbbox(ch)
    ink_cy = (bbox[1] + bbox[3]) / 2
    letter_ink[ch] = (bbox, ink_cy)


def hex_to_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def render_icon(colors_hex, border_hex):
    colors = [hex_to_rgb(c) for c in colors_hex]
    border_color = hex_to_rgb(border_hex)

    bg = Image.new("RGBA", (CONTENT, CONTENT), (255, 255, 255, 255))
    bg.putalpha(mask)

    canvas = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))

    for i, (letter, color, h_frac) in enumerate(zip(letters, colors, heights)):
        x = start_x + i * book_w
        book_h = int(CONTENT * h_frac)
        top_y = bottom_y - book_h

        shadow = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
        sd = ImageDraw.Draw(shadow)
        sd.rounded_rectangle(
            [(x + 2, bottom_y + 2), (x + book_w + 2, bottom_y + 8)],
            radius=3,
            fill=(0, 0, 0, 50),
        )
        shadow = shadow.filter(ImageFilter.GaussianBlur(6))
        canvas = Image.alpha_composite(canvas, shadow)

        spine = Image.new("RGBA", (book_w, book_h), (0, 0, 0, 0))
        sp_px = spine.load()
        for yy in range(book_h):
            for xx in range(book_w):
                vert = 0.80 + 0.20 * (1 - yy / book_h)
                horiz = 0.92 + 0.08 * (1 - abs(xx - book_w / 2) / (book_w / 2))
                factor = vert * horiz
                r = min(255, int(color[0] * factor))
                g = min(255, int(color[1] * factor))
                b = min(255, int(color[2] * factor))
                sp_px[xx, yy] = (r, g, b, 255)

        random.seed(42 + i)
        npx = spine.load()
        for yy in range(book_h):
            for xx in range(book_w):
                n_val = random.randint(-12, 12)
                r, g, b, a = npx[xx, yy]
                npx[xx, yy] = (
                    max(0, min(255, r + n_val)),
                    max(0, min(255, g + n_val)),
                    max(0, min(255, b + n_val)),
                    255,
                )

        band = ImageDraw.Draw(spine)
        book_center_y = top_y + book_h // 2
        bot_band_abs = gold_bottom_y
        top_band_abs = 2 * book_center_y - bot_band_abs
        top_band_rel = top_band_abs - top_y
        bot_band_rel = bot_band_abs - top_y
        gold = (220, 185, 100, 230)
        gold_dark = (180, 145, 70, 230)

        if 0 <= top_band_rel < book_h:
            band.rectangle(
                [(0, top_band_rel), (book_w - 1, top_band_rel + 3)], fill=gold
            )
            band.rectangle(
                [(0, top_band_rel + 5), (book_w - 1, top_band_rel + 6)], fill=gold_dark
            )
        if 0 <= bot_band_rel < book_h:
            band.rectangle(
                [(0, bot_band_rel), (book_w - 1, bot_band_rel + 3)], fill=gold
            )
            band.rectangle(
                [(0, bot_band_rel + 5), (book_w - 1, bot_band_rel + 6)], fill=gold_dark
            )

        # Shared baseline for all letters (typeset alignment): "ms" anchors
        # every glyph on one baseline, so ascenders (d, t), dotted i, and
        # x-height letters (r, c) read level and equal-sized.
        ly_rel = letter_baseline_y - top_y

        band.text(
            (book_w // 2 + 1, ly_rel + 1),
            letter,
            font=font,
            fill=(0, 0, 0, 100),
            anchor="ms",
        )
        band.text(
            (book_w // 2, ly_rel),
            letter,
            font=font,
            fill=(255, 255, 255, 245),
            anchor="ms",
        )

        spine_mask = Image.new("L", (book_w, book_h), 0)
        sm = ImageDraw.Draw(spine_mask)
        sm.rounded_rectangle([(0, 0), (book_w - 1, book_h - 1)], radius=4, fill=255)
        spine.putalpha(spine_mask)
        canvas.paste(spine, (x, top_y), spine)

    icon = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
    icon.paste(bg, (0, 0), bg)
    icon = Image.alpha_composite(icon, canvas)

    # Border with custom color
    border = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
    bd = ImageDraw.Draw(border)
    r = int(CONTENT * 0.2237)
    bd.rounded_rectangle(
        [(0, 0), (CONTENT - 1, CONTENT - 1)],
        radius=r,
        outline=(*border_color, 255),
        width=28,
    )
    border_alpha = border.split()[3]
    border.putalpha(ImageChops.multiply(border_alpha, mask))
    icon = Image.alpha_composite(icon, border)

    final = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    final.paste(icon, (PAD, PAD), icon)
    return final


# ── User-specified schemes ───────────────────────────────────────────

schemes = {
    "1_fresh_vibrant": {
        "books": ["#22C55E", "#3B82F6", "#F9C20B", "#F87171", "#8B5CF6"],
        "border": "#22C55E",
    },
    "2_blue_green_tech": {
        "books": ["#2455A4", "#3B82F6", "#14B8A6", "#86E3C0", "#A78BFA"],
        "border": "#3B82F6",
    },
    "3_warm_lively": {
        "books": ["#EF4444", "#F97316", "#FACC15", "#22C55E", "#0F9D94"],
        "border": "#F97316",
    },
    "4_premium_calm": {
        "books": ["#1E3A5F", "#64748B", "#A8B5A8", "#E8E0D0", "#8B5E3C"],
        "border": "#E9DCC6",
    },
    "5_macaron_soft": {
        "books": ["#F472B6", "#FB9A7A", "#FCD34D", "#34D399", "#A78BFA"],
        "border": "#C084FC",
    },
    "6_runer_teal": {
        "books": ["#0F9FA8", "#22B8CF", "#34D399", "#A3E635", "#ECFDF5"],
        "border": "#22C55E",
    },
}

for name, scheme in schemes.items():
    img = render_icon(scheme["books"], scheme["border"])
    path = f"/tmp/rdict_{name}.png"
    img.save(path, "PNG")
    print(f"Saved: {path}")
