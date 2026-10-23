#!/usr/bin/env python3
"""Refined version of the ORIGINAL rdict icon (5 book spines spelling
rdict). Keeps the concept; polishes: thinner warm border instead of the
heavy black one, paper-warm background, richer leather gradients, softer
modern shadows, refined gold bands.
"""

from PIL import Image, ImageDraw, ImageFont, ImageFilter, ImageChops
import random

SIZE = 1024
CONTENT = int(SIZE * 0.80)
PAD = (SIZE - CONTENT) // 2


def rounded_rect_mask(size: int) -> Image.Image:
    r = int(size * 0.2237)
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle([(0, 0), (size - 1, size - 1)], radius=r, fill=255)
    return mask


mask = rounded_rect_mask(CONTENT)


def get_font(size: int) -> ImageFont.FreeTypeFont:
    for p in [
        "/System/Library/Fonts/SF-Pro.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/Library/Fonts/SF-Pro.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]:
        try:
            return ImageFont.truetype(p, size)
        except Exception:
            continue
    return ImageFont.load_default()


letters = list("rdict")
# Tuned book colors — deep, saturated leather tones that read as a set
colors = [
    (178, 60, 62),  # r — oxblood red
    (52, 84, 150),  # d — library blue
    (196, 138, 52),  # i — warm amber
    (58, 122, 88),  # c — forest green
    (110, 74, 148),  # t — plum
]
heights = [0.50, 0.60, 0.45, 0.66, 0.55]

num_books = 5
book_w = int(CONTENT * 0.14)
gap = 0
total_w = num_books * (book_w + gap) - gap
start_x = (CONTENT - total_w) // 2
bottom_y = int(CONTENT * 0.80)

min_book_h = int(CONTENT * min(heights))
letter_center_y = bottom_y - int(min_book_h * 0.48)
gold_bottom_y = bottom_y - int(CONTENT * 0.055)

font = get_font(int(CONTENT * 0.15))

# ── Background: warm paper, not stark white ───────────────────────────

bg = Image.new("RGBA", (CONTENT, CONTENT), (252, 249, 243, 255))
# faint vignette: slightly darker at edges for depth
vig = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
vd = ImageDraw.Draw(vig)
vd.rectangle([(0, 0), (CONTENT, CONTENT)], fill=(120, 100, 70, 22))
blur = vig.filter(ImageFilter.GaussianBlur(120))
inner = Image.new("L", (CONTENT, CONTENT), 0)
idr = ImageDraw.Draw(inner)
idr.ellipse(
    [CONTENT * -0.15, CONTENT * -0.15, CONTENT * 1.15, CONTENT * 1.15],
    fill=255,
)
inner = inner.filter(ImageFilter.GaussianBlur(80))
vig.putalpha(ImageChops.multiply(blur.split()[3], inner))
bg = Image.alpha_composite(bg, vig)
bg.putalpha(mask)

canvas = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))

# ── Shared ground shadow (one soft shadow under the whole row) ────────

gshadow = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
gs = ImageDraw.Draw(gshadow)
gs.rounded_rectangle(
    [
        (start_x + 6, bottom_y + 4),
        (start_x + total_w + 10, bottom_y + 22),
    ],
    radius=14,
    fill=(40, 30, 20, 60),
)
gshadow = gshadow.filter(ImageFilter.GaussianBlur(14))
canvas = Image.alpha_composite(canvas, gshadow)

for i, (letter, color, h_frac) in enumerate(zip(letters, colors, heights)):
    x = start_x + i * (book_w + gap)
    book_h = int(CONTENT * h_frac)
    top_y = bottom_y - book_h

    # ── Leather spine: vertical gradient + center sheen ───────────────
    spine = Image.new("RGBA", (book_w, book_h), (0, 0, 0, 0))
    sp = Image.new("RGBA", (book_w, book_h), (0, 0, 0, 255))
    sp_px = sp.load()
    for yy in range(book_h):
        # leather: light from top-left, darker at bottom
        vert = 0.88 + 0.16 * (1 - yy / book_h)
        for xx in range(book_w):
            horiz = 0.94 + 0.09 * (1 - abs(xx - book_w * 0.38) / (book_w * 0.62))
            f = vert * horiz
            sp_px[xx, yy] = (
                min(255, int(color[0] * f)),
                min(255, int(color[1] * f)),
                min(255, int(color[2] * f)),
                255,
            )

    # fine grain (lighter touch than before)
    random.seed(42 + i)
    grain = Image.new("RGBA", (book_w, book_h), (0, 0, 0, 0))
    gpx = grain.load()
    for yy in range(book_h):
        for xx in range(book_w):
            n = random.randint(-7, 7)
            r, g, b, a = sp_px[xx, yy]
            gpx[xx, yy] = (
                max(0, min(255, r + n)),
                max(0, min(255, g + n)),
                max(0, min(255, b + n)),
                255,
            )
    spine = Image.alpha_composite(sp, grain)

    # left edge highlight (spine ridge) + right edge darkening
    edge = ImageDraw.Draw(spine)
    edge.rectangle([(0, 0), (3, book_h - 1)], fill=(255, 255, 255, 38))
    edge.rectangle([(book_w - 4, 0), (book_w - 1, book_h - 1)], fill=(0, 0, 0, 46))

    band = ImageDraw.Draw(spine)

    # ── Gold bands: aligned bottom, mirrored top ──────────────────────
    bot_band_abs = gold_bottom_y
    book_center_y = top_y + book_h // 2
    top_band_abs = 2 * book_center_y - bot_band_abs
    top_band_rel = top_band_abs - top_y
    bot_band_rel = bot_band_abs - top_y

    gold = (226, 192, 110, 235)
    gold_dark = (176, 140, 66, 235)

    if 0 <= top_band_rel < book_h:
        band.rectangle([(0, top_band_rel), (book_w - 1, top_band_rel + 2)], fill=gold)
        band.rectangle(
            [(0, top_band_rel + 4), (book_w - 1, top_band_rel + 5)],
            fill=gold_dark,
        )
    if 0 <= bot_band_rel < book_h:
        band.rectangle([(0, bot_band_rel), (book_w - 1, bot_band_rel + 2)], fill=gold)
        band.rectangle(
            [(0, bot_band_rel + 4), (book_w - 1, bot_band_rel + 5)],
            fill=gold_dark,
        )

    # ── Letter: embossed white with soft shadow ───────────────────────
    letter_y_rel = letter_center_y - top_y
    lsh = Image.new("RGBA", (book_w, book_h), (0, 0, 0, 0))
    lsd = ImageDraw.Draw(lsh)
    lsd.text(
        (book_w // 2 + 1, letter_y_rel + 2),
        letter,
        font=font,
        fill=(0, 0, 0, 90),
        anchor="mm",
    )
    lsh = lsh.filter(ImageFilter.GaussianBlur(1.5))
    spine = Image.alpha_composite(spine, lsh)
    band = ImageDraw.Draw(spine)
    band.text(
        (book_w // 2, letter_y_rel),
        letter,
        font=font,
        fill=(255, 253, 248, 250),
        anchor="mm",
    )

    # rounded top corners only
    spine_mask = Image.new("L", (book_w, book_h), 0)
    sm = ImageDraw.Draw(spine_mask)
    sm.rounded_rectangle([(0, 0), (book_w - 1, book_h - 1)], radius=5, fill=255)
    spine.putalpha(spine_mask)

    canvas.paste(spine, (x, top_y), spine)

# ── Composite ─────────────────────────────────────────────────────────

icon = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
icon.paste(bg, (0, 0), bg)
icon = Image.alpha_composite(icon, canvas)

# ── Border: THIN warm dark line, replaces the old 28px black slab ─────

border = Image.new("RGBA", (CONTENT, CONTENT), (0, 0, 0, 0))
bd = ImageDraw.Draw(border)
r = int(CONTENT * 0.2237)
bd.rounded_rectangle(
    [(1, 1), (CONTENT - 2, CONTENT - 2)],
    radius=r,
    outline=(74, 58, 40, 200),
    width=4,
)
# top-edge light for a subtle bevel
bd.rounded_rectangle(
    [(2, 2), (CONTENT - 3, CONTENT - 3)],
    radius=r,
    outline=(255, 255, 255, 60),
    width=1,
)
border_alpha = border.split()[3]
border.putalpha(ImageChops.multiply(border_alpha, mask))
icon = Image.alpha_composite(icon, border)

# ── Final ─────────────────────────────────────────────────────────────

final = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
final.paste(icon, (PAD, PAD), icon)

for name, img in [
    ("/tmp/icon_refined.png", final),
    ("/tmp/icon_refined_128.png", final.resize((128, 128), Image.LANCZOS)),
]:
    img.save(name)
    print(name)
