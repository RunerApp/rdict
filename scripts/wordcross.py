#!/usr/bin/env python3
"""Word-cross puzzle generator.

Randomly picks words from dictionary.db and generates crossword-style
puzzles using a backtracking placement algorithm. Clues are English
definitions from the dictionary.

Usage:
  python3 wordcross.py [seed] [count]        # ASCII preview in terminal
  python3 wordcross.py --html out.html       # playable self-contained web page
"""

import argparse
import json
import random
import sqlite3

DB_PATH = "dictionary.db"

BLOCK = "·"


def pick_candidates(conn, pool_size=300, min_len=4, max_len=9):
    """Randomly sample words that are pure a-z and have an English definition."""
    rows = conn.execute(
        """
        SELECT DISTINCT e.headword, d.value
        FROM entries e
        JOIN senses s ON s.entry_id = e.id
        JOIN definitions d ON d.sense_id = s.id
        WHERE e.lang = 'en'
          AND e.headword GLOB '[a-z]*'
          AND LENGTH(e.headword) BETWEEN ? AND ?
        ORDER BY RANDOM()
        LIMIT ?
        """,
        (min_len, max_len, pool_size * 4),
    ).fetchall()
    seen = {}
    for word, definition in rows:
        if word.isalpha() and word.isascii() and word not in seen:
            seen[word] = definition
        if len(seen) >= pool_size:
            break
    return seen


class Grid:
    def __init__(self):
        self.cells = {}          # (x, y) -> char
        self.placements = []     # (word, clue, x, y, horiz)

    def clone(self):
        g = Grid()
        g.cells = dict(self.cells)
        g.placements = list(self.placements)
        return g

    def can_place(self, word, x, y, horiz):
        """Return number of intersections if placement is legal, else -1."""
        dx, dy = (1, 0) if horiz else (0, 1)
        before = (x - dx, y - dy)
        after = (x + dx * len(word), y + dy * len(word))
        if before in self.cells or after in self.cells:
            return -1
        crosses = 0
        for i, ch in enumerate(word):
            pos = (x + dx * i, y + dy * i)
            if pos in self.cells:
                if self.cells[pos] != ch:
                    return -1
                crosses += 1
            else:
                # perpendicular neighbours must be empty
                if horiz:
                    n1, n2 = (pos[0], pos[1] - 1), (pos[0], pos[1] + 1)
                else:
                    n1, n2 = (pos[0] - 1, pos[1]), (pos[0] + 1, pos[1])
                if n1 in self.cells or n2 in self.cells:
                    return -1
        return crosses

    def place(self, word, clue, x, y, horiz):
        dx, dy = (1, 0) if horiz else (0, 1)
        for i, ch in enumerate(word):
            self.cells[(x + dx * i, y + dy * i)] = ch
        self.placements.append((word, clue, x, y, horiz))

    def bounds(self):
        xs = [p[0] for p in self.cells]
        ys = [p[1] for p in self.cells]
        return min(xs), min(ys), max(xs), max(ys)

    def area(self):
        x0, y0, x1, y1 = self.bounds()
        return (x1 - x0 + 1) * (y1 - y0 + 1)


def generate(words, target_count=12, restarts=30, seed=None):
    """Backtracking generation with random restarts; keep the best layout."""
    rng = random.Random(seed)
    items = sorted(words.items(), key=lambda kv: -len(kv[0]))
    best = None
    for _ in range(restarts):
        order = items[:]
        rng.shuffle(order)
        # keep roughly longest-first ordering with jitter for variety
        order.sort(key=lambda kv: -(len(kv[0]) + rng.random() * 4))
        grid = Grid()
        first_word, first_clue = order[0]
        grid.place(first_word, first_clue, 0, 0, True)
        placed = 1
        for word, clue in order[1:]:
            if placed >= target_count:
                break
            options = []
            for pword, _, px, py, phoriz in grid.placements:
                for i, ch in enumerate(pword):
                    for j, wch in enumerate(word):
                        if ch != wch:
                            continue
                        if phoriz:
                            x, y, horiz = px + i, py - j, False
                        else:
                            x, y, horiz = px - j, py + i, True
                        crosses = grid.can_place(word, x, y, horiz)
                        if crosses > 0:
                            options.append((crosses, rng.random(), x, y, horiz))
            if options:
                options.sort(reverse=True)
                _, _, x, y, horiz = options[0]
                grid.place(word, clue, x, y, horiz)
                placed += 1
        if best is None or len(grid.placements) > len(best.placements) or (
            len(grid.placements) == len(best.placements) and grid.area() < best.area()
        ):
            best = grid
    return best


def build_puzzle(grid):
    """Convert a Grid into a JSON-friendly puzzle dict with normalized coords."""
    x0, y0, x1, y1 = grid.bounds()
    w, h = x1 - x0 + 1, y1 - y0 + 1
    g = [[None] * w for _ in range(h)]
    for (x, y), ch in grid.cells.items():
        g[y - y0][x - x0] = ch.upper()
    numbers = {}
    for _, _, x, y, _ in sorted(grid.placements, key=lambda p: (p[3] - y0, p[2] - x0)):
        key = (x - x0, y - y0)
        if key not in numbers:
            numbers[key] = len(numbers) + 1
    words = []
    for word, clue, x, y, horiz in grid.placements:
        words.append({
            "n": numbers[(x - x0, y - y0)],
            "d": "A" if horiz else "D",
            "x": x - x0,
            "y": y - y0,
            "len": len(word),
            "word": word.upper(),
            "clue": clue,
        })
    words.sort(key=lambda t: (t["d"], t["n"]))
    return {"w": w, "h": h, "grid": g, "words": words}


def render(grid):
    x0, y0, x1, y1 = grid.bounds()
    width, height = x1 - x0 + 1, y1 - y0 + 1
    char_grid = [[BLOCK] * width for _ in range(height)]
    for (x, y), ch in grid.cells.items():
        char_grid[y - y0][x - x0] = ch

    # number the starting cell of each placement (top-to-bottom, left-to-right)
    numbers = {}
    for _, _, x, y, _ in sorted(grid.placements, key=lambda p: (p[3] - y0, p[2] - x0)):
        key = (x - x0, y - y0)
        if key not in numbers:
            numbers[key] = len(numbers) + 1

    # two aligned views: puzzle (blank cells to fill) and answer key
    puzzle = []
    answer = []
    for row_idx, row in enumerate(char_grid):
        for target, show_letters in ((puzzle, False), (answer, True)):
            line = f"{row_idx:>2} "
            for col_idx, ch in enumerate(row):
                n = numbers.get((col_idx, row_idx))
                cell = ch if (show_letters and ch != BLOCK) else " "
                line += (f"[{n:>2}]" if n else "    ") + cell
            target.append(line + " ")
    lines = ["Puzzle:", *puzzle, "", "Answer:", *answer]

    clues = {"Across": [], "Down": []}
    for word, clue, x, y, horiz in grid.placements:
        n = numbers[(x - x0, y - y0)]
        clues["Across" if horiz else "Down"].append((n, word, clue))
    for k in clues:
        clues[k].sort()

    return "\n".join(lines), clues


HTML_TEMPLATE = r"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Word Cross</title>
<style>
  * { box-sizing: border-box; }
  body {
    margin: 0; font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    background: #f2efe6; color: #222;
  }
  header {
    padding: 18px 20px 6px; max-width: 1060px; margin: 0 auto;
  }
  header h1 { margin: 0; font-size: 26px; letter-spacing: 0.5px; }
  header p { margin: 4px 0 0; color: #777; font-size: 13px; }
  main {
    display: flex; flex-wrap: wrap; gap: 22px;
    padding: 12px 20px 32px; max-width: 1060px; margin: 0 auto;
  }
  #gridwrap { flex: 1 1 380px; max-width: 620px; }
  #grid {
    display: grid; gap: 2px; padding: 2px; background: #23201a;
    border-radius: 8px; width: 100%; user-select: none; touch-action: manipulation;
  }
  .cell {
    position: relative; background: #fff; aspect-ratio: 1;
    display: flex; align-items: center; justify-content: center;
    font-size: calc(var(--cs, 30px) * 0.55); font-weight: 700; cursor: pointer;
  }
  .block { background: #23201a; aspect-ratio: 1; }
  .num {
    position: absolute; top: 2px; left: 3px;
    font-size: calc(var(--cs, 30px) * 0.26); font-weight: 600; color: #8a8a8a;
  }
  .letter { line-height: 1; text-transform: uppercase; }
  .cell.inword { background: #f4ecc8; }
  .cell.sel { background: #f0cf5a; }
  .cell.bad .letter { color: #c0392b; }
  .cell.rev .letter { color: #2471a3; }
  #controls { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-top: 14px; }
  button {
    font: inherit; font-size: 14px; padding: 7px 14px; border-radius: 7px;
    border: 1px solid #c9c2ad; background: #fff; cursor: pointer;
  }
  button:hover { background: #faf7ee; }
  button.primary { background: #3d5a80; border-color: #3d5a80; color: #fff; }
  button.primary:hover { background: #33496b; }
  #dir {
    margin-left: auto; font-size: 14px; color: #555;
    background: #fff; border: 1px solid #d8d2be; padding: 7px 12px; border-radius: 7px;
  }
  #status { min-height: 1.5em; margin-top: 10px; font-size: 14px; color: #666; }
  #status.win { color: #2e7d32; font-weight: 700; font-size: 16px; }
  #clues { flex: 1 1 300px; min-width: 280px; max-height: 82vh; overflow: auto; }
  #clues h3 {
    margin: 4px 0 8px; font-size: 15px; text-transform: uppercase;
    letter-spacing: 1px; color: #555;
  }
  #clues ol { list-style: none; margin: 0 0 18px; padding: 0; }
  #clues li {
    padding: 6px 9px; border-radius: 6px; cursor: pointer;
    font-size: 14px; line-height: 1.45;
  }
  #clues li:hover { background: #e9e4d4; }
  #clues li.active { background: #f0cf5a; }
  #clues li .n { font-weight: 700; margin-right: 2px; }
  #clues li .len { color: #999; font-size: 12px; }
  #kb { position: fixed; left: -100px; top: 0; width: 20px; height: 20px; opacity: 0; }
  footer {
    max-width: 1060px; margin: 0 auto; padding: 0 20px 24px;
    color: #999; font-size: 12px;
  }
</style>
</head>
<body>
<header>
  <h1>Word Cross</h1>
  <p id="subtitle">Fill the grid from the English definitions.</p>
</header>
<main>
  <section id="gridwrap">
    <div id="grid"></div>
    <div id="controls">
      <button id="btnNew" class="primary">New Puzzle</button>
      <button id="btnCheck">Check</button>
      <button id="btnReveal">Reveal</button>
      <button id="btnClear">Clear</button>
      <span id="dir"></span>
    </div>
    <div id="status"></div>
  </section>
  <aside id="clues">
    <h3>Across</h3>
    <ol id="across"></ol>
    <h3>Down</h3>
    <ol id="down"></ol>
  </aside>
</main>
<footer>Puzzles generated from your local dictionary.db. Type letters, click clues, arrow keys to move, space to switch direction.</footer>
<input id="kb" autocomplete="off" autocorrect="off" autocapitalize="characters" spellcheck="false">
<script>
'use strict';
var DATA = __PUZZLE_DATA__;
var gridEl = document.getElementById('grid');
var kb = document.getElementById('kb');
var acrossList = document.getElementById('across');
var downList = document.getElementById('down');
var statusEl = document.getElementById('status');
var dirEl = document.getElementById('dir');
var subtitleEl = document.getElementById('subtitle');

var P, user, cellEls, numAt, clueEls;
var idx = 0;
var cur = { x: 0, y: 0, dir: 'A' };

function inBounds(x, y) { return x >= 0 && y >= 0 && x < P.w && y < P.h; }
function isCell(x, y) { return inBounds(x, y) && P.grid[y][x] !== null; }

function wordAt(x, y, dir) {
  for (var i = 0; i < P.words.length; i++) {
    var w = P.words[i];
    if (w.d !== dir) continue;
    if (dir === 'A' ? (y === w.y && x >= w.x && x < w.x + w.len)
                    : (x === w.x && y >= w.y && y < w.y + w.len)) return w;
  }
  return null;
}

function loadPuzzle(i) {
  P = DATA.puzzles[i % DATA.puzzles.length];
  user = P.grid.map(function (row) { return row.map(function () { return null; }); });
  cur = { x: P.words[0].x, y: P.words[0].y, dir: P.words[0].d };
  numAt = {};
  for (var i2 = 0; i2 < P.words.length; i2++) {
    var w = P.words[i2];
    var k = w.y * P.w + w.x;
    if (!(k in numAt)) numAt[k] = w.n;
  }
  renderGrid();
  renderClues();
  statusEl.textContent = '';
  statusEl.classList.remove('win');
  subtitleEl.textContent = 'Puzzle ' + ((i % DATA.puzzles.length) + 1) + ' of ' +
    DATA.puzzles.length + ' — fill the grid from the English definitions.';
  update();
  kb.focus({ preventScroll: true });
}

function renderGrid() {
  gridEl.innerHTML = '';
  gridEl.style.gridTemplateColumns = 'repeat(' + P.w + ', 1fr)';
  cellEls = [];
  for (var y = 0; y < P.h; y++) {
    cellEls.push([]);
    for (var x = 0; x < P.w; x++) {
      if (P.grid[y][x] === null) {
        var b = document.createElement('div');
        b.className = 'block';
        gridEl.appendChild(b);
        cellEls[y].push(null);
      } else {
        var c = document.createElement('div');
        c.className = 'cell';
        var n = numAt[y * P.w + x];
        if (n) {
          var s = document.createElement('span');
          s.className = 'num';
          s.textContent = n;
          c.appendChild(s);
        }
        var l = document.createElement('span');
        l.className = 'letter';
        c.appendChild(l);
        (function (x2, y2) {
          c.addEventListener('mousedown', function (e) { e.preventDefault(); });
          c.addEventListener('click', function () { onCellClick(x2, y2); });
        })(x, y);
        gridEl.appendChild(c);
        cellEls[y].push(c);
      }
    }
  }
  fitFont();
}

function fitFont() {
  var cs = (gridEl.clientWidth - 4) / P.w;
  gridEl.style.setProperty('--cs', cs + 'px');
}
window.addEventListener('resize', fitFont);

function renderClues() {
  acrossList.innerHTML = '';
  downList.innerHTML = '';
  clueEls = {};
  for (var i = 0; i < P.words.length; i++) {
    var w = P.words[i];
    var li = document.createElement('li');
    var nSpan = document.createElement('span');
    nSpan.className = 'n';
    nSpan.textContent = w.n + '.';
    var lenSpan = document.createElement('span');
    lenSpan.className = 'len';
    lenSpan.textContent = ' (' + w.len + ')';
    li.appendChild(nSpan);
    li.appendChild(document.createTextNode(w.clue));
    li.appendChild(lenSpan);
    (function (w2) {
      li.addEventListener('mousedown', function (e) { e.preventDefault(); });
      li.addEventListener('click', function () { selectWord(w2); });
    })(w);
    (w.d === 'A' ? acrossList : downList).appendChild(li);
    clueEls[w.d + w.n] = li;
  }
}

function onCellClick(x, y) {
  kb.focus({ preventScroll: true });
  if (cur.x === x && cur.y === y) {
    toggleDir();
  } else {
    cur.x = x; cur.y = y;
    if (!wordAt(x, y, cur.dir)) cur.dir = cur.dir === 'A' ? 'D' : 'A';
    update();
  }
}

function toggleDir() {
  var other = cur.dir === 'A' ? 'D' : 'A';
  if (wordAt(cur.x, cur.y, other)) cur.dir = other;
  update();
}

function selectWord(w) {
  kb.focus({ preventScroll: true });
  cur.dir = w.d;
  var sx = w.x, sy = w.y;
  for (var i = 0; i < w.len; i++) {
    var x = w.x + (w.d === 'A' ? i : 0);
    var y = w.y + (w.d === 'D' ? i : 0);
    if (!user[y][x]) { sx = x; sy = y; break; }
  }
  cur.x = sx; cur.y = sy;
  update();
}

function update() {
  for (var y = 0; y < P.h; y++) for (var x = 0; x < P.w; x++) {
    var c = cellEls[y][x];
    if (c) c.classList.remove('sel', 'inword');
  }
  var w = wordAt(cur.x, cur.y, cur.dir);
  if (w) {
    for (var i = 0; i < w.len; i++) {
      var x2 = w.x + (w.d === 'A' ? i : 0);
      var y2 = w.y + (w.d === 'D' ? i : 0);
      cellEls[y2][x2].classList.add('inword');
    }
  }
  cellEls[cur.y][cur.x].classList.add('sel');
  dirEl.textContent = w ? (w.d === 'A' ? '\u2192 Across' : '\u2193 Down') : '';
  for (var k in clueEls) clueEls[k].classList.remove('active');
  if (w) {
    var li = clueEls[w.d + w.n];
    li.classList.add('active');
    li.scrollIntoView({ block: 'nearest' });
  }
}

function setLetter(x, y, ch) {
  user[y][x] = ch;
  cellEls[y][x].querySelector('.letter').textContent = ch || '';
}

function move(dx, dy) {
  var x = cur.x + dx, y = cur.y + dy;
  while (inBounds(x, y) && !isCell(x, y)) { x += dx; y += dy; }
  if (isCell(x, y)) { cur.x = x; cur.y = y; update(); }
}

function advance() {
  var dx = cur.dir === 'A' ? 1 : 0, dy = cur.dir === 'D' ? 1 : 0;
  var x = cur.x + dx, y = cur.y + dy;
  if (isCell(x, y)) { cur.x = x; cur.y = y; }
}

function stepBack() {
  var dx = cur.dir === 'A' ? 1 : 0, dy = cur.dir === 'D' ? 1 : 0;
  var x = cur.x - dx, y = cur.y - dy;
  if (isCell(x, y)) { cur.x = x; cur.y = y; }
}

function handleChar(ch) {
  ch = ch.toUpperCase();
  if (!/^[A-Z]$/.test(ch) || !isCell(cur.x, cur.y)) return;
  setLetter(cur.x, cur.y, ch);
  checkWin();
  advance();
  update();
}

function doBackspace() {
  if (user[cur.y][cur.x]) {
    setLetter(cur.x, cur.y, null);
  } else {
    stepBack();
    if (user[cur.y][cur.x]) setLetter(cur.x, cur.y, null);
  }
  update();
}

function handleKey(e) {
  var k = e.key;
  if (k === 'ArrowLeft') { e.preventDefault(); cur.dir = 'A'; move(-1, 0); }
  else if (k === 'ArrowRight') { e.preventDefault(); cur.dir = 'A'; move(1, 0); }
  else if (k === 'ArrowUp') { e.preventDefault(); cur.dir = 'D'; move(0, -1); }
  else if (k === 'ArrowDown') { e.preventDefault(); cur.dir = 'D'; move(0, 1); }
  else if (k === ' ' || k === 'Enter' || k === 'Tab') { e.preventDefault(); toggleDir(); }
  else if (k === 'Backspace') { e.preventDefault(); doBackspace(); }
}

function totalCells() {
  var n = 0;
  for (var y = 0; y < P.h; y++) for (var x = 0; x < P.w; x++) if (P.grid[y][x]) n++;
  return n;
}

function checkWin() {
  for (var y = 0; y < P.h; y++) for (var x = 0; x < P.w; x++) {
    if (!P.grid[y][x]) continue;
    if (user[y][x] !== P.grid[y][x]) return false;
  }
  statusEl.textContent = 'Solved! Well played.';
  statusEl.classList.add('win');
  return true;
}

function check() {
  var bad = 0, filled = 0;
  for (var y = 0; y < P.h; y++) for (var x = 0; x < P.w; x++) {
    if (!P.grid[y][x]) continue;
    var c = cellEls[y][x];
    c.classList.remove('bad');
    if (user[y][x]) {
      filled++;
      if (user[y][x] !== P.grid[y][x]) { c.classList.add('bad'); bad++; }
    }
  }
  if (!filled) statusEl.textContent = 'Nothing filled in yet.';
  else if (bad === 0 && filled === totalCells()) { statusEl.textContent = 'Solved! Well played.'; statusEl.classList.add('win'); }
  else if (bad === 0) statusEl.textContent = 'So far so good — no mistakes.';
  else statusEl.textContent = bad + ' wrong letter' + (bad > 1 ? 's' : '') + '.';
}

function reveal() {
  for (var y = 0; y < P.h; y++) for (var x = 0; x < P.w; x++) {
    if (!P.grid[y][x]) continue;
    cellEls[y][x].classList.remove('bad');
    cellEls[y][x].classList.add('rev');
    setLetter(x, y, P.grid[y][x]);
  }
  statusEl.textContent = 'Puzzle revealed.';
}

function clearAll() {
  for (var y = 0; y < P.h; y++) for (var x = 0; x < P.w; x++) {
    if (!P.grid[y][x]) continue;
    cellEls[y][x].classList.remove('bad', 'rev');
    setLetter(x, y, null);
  }
  statusEl.textContent = '';
  statusEl.classList.remove('win');
  update();
}

document.getElementById('btnNew').addEventListener('click', function () {
  idx++;
  loadPuzzle(idx);
});
document.getElementById('btnCheck').addEventListener('click', function () { check(); kb.focus({ preventScroll: true }); });
document.getElementById('btnReveal').addEventListener('click', function () { reveal(); kb.focus({ preventScroll: true }); });
document.getElementById('btnClear').addEventListener('click', function () { clearAll(); kb.focus({ preventScroll: true }); });

kb.addEventListener('keydown', handleKey);
kb.addEventListener('input', function (e) {
  if (e.inputType === 'deleteContentBackward' && !kb.value) { doBackspace(); return; }
  var v = kb.value;
  kb.value = '';
  if (v) handleChar(v.charAt(v.length - 1));
});

loadPuzzle(0);
</script>
</body>
</html>
"""


def emit_html(puzzles, path):
    data = json.dumps({"puzzles": puzzles}, ensure_ascii=False).replace("</", "<\\/")
    html = HTML_TEMPLATE.replace("__PUZZLE_DATA__", data)
    with open(path, "w", encoding="utf-8") as f:
        f.write(html)


def main():
    ap = argparse.ArgumentParser(description="Word-cross puzzle generator")
    ap.add_argument("seed", nargs="?", type=int, default=None)
    ap.add_argument("count", nargs="?", type=int, default=12, help="words per puzzle")
    ap.add_argument("--pool", type=int, default=300, help="candidate pool size")
    ap.add_argument("--html", metavar="PATH", help="write a playable HTML page")
    ap.add_argument("--puzzles", type=int, default=5, help="number of puzzles embedded in HTML")
    args = ap.parse_args()

    conn = sqlite3.connect(DB_PATH)

    if args.html:
        master = random.Random(args.seed)
        puzzles = []
        attempts = 0
        while len(puzzles) < args.puzzles and attempts < args.puzzles * 4:
            attempts += 1
            words = pick_candidates(conn, pool_size=args.pool)
            grid = generate(words, target_count=args.count,
                            seed=master.randrange(2 ** 31))
            if grid and len(grid.placements) >= max(4, args.count // 2):
                puzzles.append(build_puzzle(grid))
        if not puzzles:
            print("failed to build any puzzle, try another seed")
            return
        emit_html(puzzles, args.html)
        print(f"wrote {args.html} with {len(puzzles)} puzzle(s)")
        return

    words = pick_candidates(conn, pool_size=args.pool)
    grid = generate(words, target_count=args.count, seed=args.seed)
    if grid is None or len(grid.placements) < 3:
        print("failed to build a puzzle, try another seed")
        return
    text, clues = render(grid)
    print(text)
    print()
    for direction in ("Across", "Down"):
        print(f"{direction}:")
        for n, word, clue in clues[direction]:
            print(f"  {n:>2}. {clue}  ({len(word)})")
    print()
    print(f"placed {len(grid.placements)}/{len(words)} candidate words")


if __name__ == "__main__":
    main()
