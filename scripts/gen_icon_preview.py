#!/usr/bin/env python3
"""Generate 3 rdict app icon design proposals for review (1024px + 128px
dock previews). Not the production generator — see gen_icon.py."""

from PIL import Image, ImageDraw, ImageFont, ImageFilter

SIZE = 1024
# rdict language palette (covers/SPEC.md v2)
NAVY = (20, 33, 61)       # #14213d Oxford navy
CINNABAR = (143, 29, 33)  # #8f1d21 cinnabar red
GRAPHITE = (43, 47, 51)   # #2b2f33 metadata bar
IVORY = (253, 251, 247)   # #fdfbf7 ink
OCHRE = (107, 66, 38)     # #6b4226 es ochre (book B)


def rounded_rect_mask(size: int) -> Image.Image:
    r = int(size * 0.2237)
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle([(0, 0), (size - 1, size - 1)], radius=r, fill=255)
    return mask


def get_font(size: int, serif: bool = False, mono: bool = False):
    candidates = (
        [
            "/System/Library/Fonts/Supplemental/Georgia Bold.ttf",
            "/System/Library/Fonts/Supplemental/Georgia.ttf",
        ]
        if serif
        else [
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/SFMono-Regular.otf",
        ]
        if mono
        else [
            "/System/Library/Fonts/SF-Pro.ttf",
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
    )
    for p in candidates:
        try:
            return ImageFont.truetype(p, size)
        except Exception:
            continue
    return ImageFont.load_default()


def new_icon():
    icon = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    return icon, ImageDraw.Draw(icon)


def apply_mask(icon: Image.Image) -> Image.Image:
    out = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    out.paste(icon, (0, 0), rounded_rect_mask(SIZE))
    return out


def subtle_vgrad(w, h, top, bottom):
    """Vertical gradient image from top to bottom color."""
    img = Image.new("RGBA", (w, h))
    px = img.load()
    for y in range(h):
        t = y / max(1, h - 1)
        px[0, y] = (
            int(top[0] + (bottom[0] - top[0]) * t),
            int(top[1] + (bottom[1] - top[1]) * t),
            int(top[2] + (bottom[2] - top[2]) * t),
            255,
        )
    return img.resize((w, h))


# ── Proposal A: language split-panel (cover family) ───────────────────


def proposal_a():
    icon, d = new_icon()
    meta_h = int(SIZE * 0.07)
    panel_h = (SIZE - meta_h) // 2

    # subtle vertical gradients within each panel for depth
    top = subtle_vgrad(SIZE, panel_h, (26, 43, 79), NAVY)
    bot = subtle_vgrad(SIZE, panel_h, CINNABAR, (110, 22, 26))
    icon.paste(top, (0, 0))
    icon.paste(bot, (0, panel_h))
    d.rectangle([(0, SIZE - meta_h), (SIZE, SIZE)], fill=GRAPHITE)

    font = get_font(int(SIZE * 0.28), mono=True)
    # "En" in top panel, "中" in bottom panel
    d.text(
        (SIZE // 2, panel_h // 2),
        "En",
        font=font,
        fill=IVORY,
        anchor="mm",
    )
    # CJK glyph needs a CJK font
    cjk = None
    for p in [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
    ]:
        try:
            cjk = ImageFont.truetype(p, int(SIZE * 0.28))
            break
        except Exception:
            continue
    d.text(
        (SIZE // 2, panel_h + panel_h // 2),
        "中",
        font=cjk or font,
        fill=IVORY,
        anchor="mm",
    )
    return apply_mask(icon)


# ── Proposal B: modern book stack ─────────────────────────────────────


def proposal_b():
    icon, d = new_icon()
    # ivory background with a soft radial-ish center light
    bg = Image.new("RGBA", (SIZE, SIZE), IVORY + (255,))
    light = Image.new("RGBA", (SIZE, SIZE), (255, 255, 255, 60))
    lmask = Image.new("L", (SIZE, SIZE), 0)
    lm = ImageDraw.Draw(lmask)
    lm.ellipse(
        [SIZE * 0.1, SIZE * 0.05, SIZE * 0.9, SIZE * 0.85], fill=90
    )
    light.putalpha(Image.eval(lmask, lambda v: v))
    bg = Image.alpha_composite(bg, light)
    icon = bg

    # two standing books at the back (ochre + navy), front-facing book
    # center-front with big serif R.
    ground_y = int(SIZE * 0.78)

    def standing_book(x, w, h, color):
        book = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        grad = subtle_vgrad(w, h, color, tuple(int(c * 0.72) for c in color))
        book.paste(grad, (0, 0))
        bd = ImageDraw.Draw(book)
        # spine pages hint on the right edge
        bd.rectangle([(w - int(w * 0.16), 0), (w, h)], fill=IVORY + (235,))
        bd.rectangle(
            [(w - int(w * 0.16), 0), (w - int(w * 0.16) + 4, h)],
            fill=(0, 0, 0, 40),
        )
        # rounded top
        m = Image.new("L", (w, h), 0)
        mm = ImageDraw.Draw(m)
        mm.rounded_rectangle([(0, 0), (w - 1, h - 1)], radius=14, fill=255)
        book.putalpha(m)
        return book

    # drop shadows on ground
    shadow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    sd.ellipse(
        [SIZE * 0.16, ground_y - 26, SIZE * 0.84, ground_y + 40],
        fill=(0, 0, 0, 46),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(22))
    icon = Image.alpha_composite(icon, shadow)

    # back-left standing book (ochre)
    bw = int(SIZE * 0.14)
    b1 = standing_book(int(SIZE * 0.20), bw, int(SIZE * 0.42), OCHRE)
    # back-right standing book (navy)
    b2 = standing_book(int(SIZE * 0.66), bw, int(SIZE * 0.36), NAVY)
    icon.paste(b1, (int(SIZE * 0.20), ground_y - b1.height), b1)
    icon.paste(b2, (int(SIZE * 0.66), ground_y - b2.height), b2)

    # front-facing book: cinnabar cover with big serif R
    fw, fh = int(SIZE * 0.46), int(SIZE * 0.56)
    fx, fy = (SIZE - fw) // 2, ground_y - fh
    front = Image.new("RGBA", (fw, fh), (0, 0, 0, 0))
    grad = subtle_vgrad(fw, fh, tuple(min(255, c + 22) for c in CINNABAR), CINNABAR)
    front.paste(grad, (0, 0))
    fd = ImageDraw.Draw(front)
    # gold double rule frame
    gold = (220, 185, 100, 255)
    inset = int(fw * 0.09)
    fd.rectangle(
        [(inset, inset), (fw - inset, fh - inset)],
        outline=gold,
        width=5,
    )
    fd.rectangle(
        [(inset + 12, inset + 12), (fw - inset - 12, fh - inset - 12)],
        outline=(220, 185, 100, 140),
        width=2,
    )
    # big serif R
    rfont = get_font(int(fh * 0.52), serif=True)
    fd.text((fw // 2, fh // 2), "R", font=rfont, fill=IVORY, anchor="mm")
    # rounded corners
    m = Image.new("L", (fw, fh), 0)
    mm = ImageDraw.Draw(m)
    mm.rounded_rectangle([(0, 0), (fw - 1, fh - 1)], radius=26, fill=255)
    front.putalpha(m)
    icon.paste(front, (fx, fy), front)

    return apply_mask(icon)


# ── Proposal C: gradient letter ───────────────────────────────────────


def proposal_c():
    icon, _ = new_icon()
    grad = subtle_vgrad(SIZE, SIZE, (26, 43, 79), (120, 24, 28))
    icon.paste(grad, (0, 0))
    d = ImageDraw.Draw(icon)

    rfont = get_font(int(SIZE * 0.58), serif=True)
    # soft shadow behind R
    sh = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    sd = ImageDraw.Draw(sh)
    sd.text((SIZE // 2 + 10, SIZE // 2 + 16), "R", font=rfont, fill=(0, 0, 0, 90), anchor="mm")
    sh = sh.filter(ImageFilter.GaussianBlur(14))
    icon = Image.alpha_composite(icon, sh)
    d = ImageDraw.Draw(icon)
    d.text((SIZE // 2, SIZE // 2), "R", font=rfont, fill=IVORY, anchor="mm")
    return apply_mask(icon)


def main():
    for name, fn in [("a", proposal_a), ("b", proposal_b), ("c", proposal_c)]:
        icon = fn()
        icon.save(f"/tmp/icon_{name}.png")
        icon.resize((128, 128), Image.LANCZOS).save(f"/tmp/icon_{name}_128.png")
        print(f"/tmp/icon_{name}.png + /tmp/icon_{name}_128.png")


if __name__ == "__main__":
    main()
