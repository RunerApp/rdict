#!/usr/bin/env python3
"""rdict app icon — VECTOR master (SVG) + rendered PNG.

Same concept as the original: five book spines spelling "rdict", gold
bands, leather tones. SVG is the master (hand-editable, resolution
independent); rsvg-convert renders PNG at any size.

Usage: gen_icon_vector.py [--install]
  default: write scripts/icon.svg, render /tmp/icon_vector.png (+128px)
  --install: also render AppIcon.png/.icns into the .app bundle
"""

import argparse
import os
import subprocess

SIZE = 1024
CONTENT = int(SIZE * 0.80)  # 819
PAD = (SIZE - CONTENT) // 2  # 102

# rdict icon palette — library leather set (pairs: light top / dark bottom)
BOOKS = [
    # letter, (light, dark),          height fraction
    ("r", ("#b23c3e", "#7c2a2c"), 0.50),  # oxblood
    ("d", ("#3a5da0", "#27396a"), 0.60),  # library blue
    ("i", ("#c68e36", "#8a6220"), 0.45),  # warm amber
    ("c", ("#3d7f5c", "#29573d"), 0.66),  # forest green
    ("t", ("#74509c", "#4e3268"), 0.55),  # plum
]
PAPER_TOP = "#fdfbf6"
PAPER_BOTTOM = "#eee7d7"
BORDER = "#4a3a28"
GOLD = "#e2c06e"
GOLD_DARK = "#b08c42"
INK = "#fffdf8"
SHADOW = "#2a2015"

BOOK_W = round(CONTENT * 0.14)  # 115
N = len(BOOKS)
TOTAL_W = N * BOOK_W
START_X = (CONTENT - TOTAL_W) // 2
BOTTOM_Y = round(CONTENT * 0.80)  # 655
MIN_H = round(CONTENT * min(b[2] for b in BOOKS))
LETTER_CY = round(BOTTOM_Y - MIN_H * 0.48)
GOLD_BOTTOM_Y = round(BOTTOM_Y - CONTENT * 0.055)

FONT = "-apple-system, 'SF Pro Text', 'Helvetica Neue', sans-serif"
FONT_SIZE = round(CONTENT * 0.15)  # ~123

LETTER_FONT = f"'Georgia', 'Times New Roman', serif"


def build_svg() -> str:
    p = []
    p.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{SIZE}"'
        f' height="{SIZE}" viewBox="0 0 {SIZE} {SIZE}">'
    )
    p.append(
        f"<defs>"
        f'<radialGradient id="paper" cx="50%" cy="40%" r="78%">'
        f'<stop offset="0" stop-color="{PAPER_TOP}"/>'
        f'<stop offset="1" stop-color="{PAPER_BOTTOM}"/>'
        f"</radialGradient>"
        f'<filter id="blur-s" x="-40%" y="-40%" width="180%" height="180%">'
        f'<feGaussianBlur stdDeviation="10"/>'
        f"</filter>"
        f'<filter id="blur-m" x="-40%" y="-40%" width="180%" height="180%">'
        f'<feGaussianBlur stdDeviation="18"/>'
        f"</filter>"
    )
    # leather gradient per book
    for i, (_, (light, dark), _) in enumerate(BOOKS):
        p.append(
            f'<linearGradient id="leather{i}" x1="0" y1="0" x2="0" y2="1">'
            f'<stop offset="0" stop-color="{light}"/>'
            f'<stop offset="1" stop-color="{dark}"/>'
            f"</linearGradient>"
        )
    p.append("</defs>")

    # squircle background
    rx = round(SIZE * 0.2237)
    p.append(f'<rect width="{SIZE}" height="{SIZE}" rx="{rx}" fill="url(#paper)"/>')

    # inner group in content coordinates
    p.append(f'<g transform="translate({PAD},{PAD})">')

    # ground shadow under the whole row
    p.append(
        f'<ellipse cx="{START_X + TOTAL_W / 2:.0f}" cy="{BOTTOM_Y + 12}"'
        f' rx="{TOTAL_W / 2 + 16:.0f}" ry="16" fill="{SHADOW}" opacity="0.20"'
        f' filter="url(#blur-m)"/>'
    )

    for i, (letter, (light, dark), h_frac) in enumerate(BOOKS):
        x = START_X + i * BOOK_W
        h = round(CONTENT * h_frac)
        top = BOTTOM_Y - h
        center_y = top + h // 2

        # spine: top-rounded rect with vertical leather gradient
        r = 6
        p.append(
            f'<path d="M{x},{top + r} a{r},{r} 0 0 1 {r},{-r}'
            f" h{BOOK_W - 2 * r} a{r},{r} 0 0 1 {r},{r}"
            f' v{h - r} h{-BOOK_W} z" fill="url(#leather{i})"/>'
        )
        # spine ridge: left highlight, right shade
        p.append(
            f'<rect x="{x}" y="{top}" width="4" height="{h}"'
            f' fill="#ffffff" opacity="0.16"/>'
        )
        p.append(
            f'<rect x="{x + BOOK_W - 5}" y="{top}" width="5" height="{h}"'
            f' fill="#000000" opacity="0.18"/>'
        )

        # gold bands: shared bottom Y, top mirrored about book center
        bot_rel = GOLD_BOTTOM_Y - top
        top_rel = 2 * center_y - GOLD_BOTTOM_Y - top
        for yr in (top_rel, bot_rel):
            if 0 <= yr < h:
                p.append(
                    f'<rect x="{x}" y="{top + yr:.0f}" width="{BOOK_W}"'
                    f' height="3" fill="{GOLD}"/>'
                )
                p.append(
                    f'<rect x="{x}" y="{top + yr + 5:.0f}" width="{BOOK_W}"'
                    f' height="2" fill="{GOLD_DARK}"/>'
                )

        # letter, embossed: soft shadow + ivory glyph, shared baseline
        ly = LETTER_CY + FONT_SIZE * 0.34
        p.append(
            f'<text x="{x + BOOK_W / 2 + 1:.0f}" y="{ly + 2:.0f}"'
            f' text-anchor="middle" font-family="{FONT}"'
            f' font-weight="600" font-size="{FONT_SIZE}" fill="#000"'
            f' opacity="0.32" filter="url(#blur-s)">{letter}</text>'
        )
        p.append(
            f'<text x="{x + BOOK_W / 2:.0f}" y="{ly:.0f}"'
            f' text-anchor="middle" font-family="{FONT}"'
            f' font-weight="600" font-size="{FONT_SIZE}" fill="{INK}">'
            f"{letter}</text>"
        )

    p.append("</g>")

    # thin warm border with a top-edge light
    inset = PAD
    p.append(
        f'<rect x="{inset + 1}" y="{inset + 1}"'
        f' width="{SIZE - 2 * inset - 2}" height="{SIZE - 2 * inset - 2}"'
        f' rx="{rx}" fill="none" stroke="{BORDER}" stroke-width="4"'
        f' opacity="0.85"/>'
    )
    p.append(
        f'<rect x="{inset + 3}" y="{inset + 3}"'
        f' width="{SIZE - 2 * inset - 6}" height="{SIZE - 2 * inset - 6}"'
        f' rx="{rx}" fill="none" stroke="#ffffff" stroke-width="1"'
        f' opacity="0.35"/>'
    )

    p.append("</svg>")
    return "\n".join(p)


def render(svg_path: str, out_png: str, size: int):
    subprocess.run(
        ["rsvg-convert", "-w", str(size), "-h", str(size), "-o", out_png, svg_path],
        check=True,
    )


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install", action="store_true")
    args = ap.parse_args()

    here = os.path.dirname(os.path.abspath(__file__))
    svg_path = os.path.join(here, "icon.svg")
    svg = build_svg()
    with open(svg_path, "w", encoding="utf-8") as f:
        f.write(svg)
    print(f"master: {svg_path}")

    render(svg_path, "/tmp/icon_vector.png", SIZE)
    render(svg_path, "/tmp/icon_vector_128.png", 128)
    print("/tmp/icon_vector.png + /tmp/icon_vector_128.png")

    if args.install:
        out_dir = os.path.abspath(
            os.path.join(
                here, "..", "apps", "apple", "RdictApp.app", "Contents", "Resources"
            )
        )
        os.makedirs(out_dir, exist_ok=True)
        render(svg_path, os.path.join(out_dir, "AppIcon.png"), SIZE)
        iconset = os.path.join(out_dir, "AppIcon.iconset")
        os.makedirs(iconset, exist_ok=True)
        for s in [16, 32, 64, 128, 256, 512, 1024]:
            render(svg_path, os.path.join(iconset, f"icon_{s}x{s}.png"), s)
            if s <= 512:
                render(svg_path, os.path.join(iconset, f"icon_{s}x{s}@2x.png"), s * 2)
        subprocess.run(
            [
                "iconutil",
                "-c",
                "icns",
                iconset,
                "-o",
                os.path.join(out_dir, "AppIcon.icns"),
            ],
            check=True,
            capture_output=True,
        )
        import shutil

        shutil.rmtree(iconset)
        print(f"installed: {out_dir}/AppIcon.png + AppIcon.icns")


if __name__ == "__main__":
    main()
