import sqlite3, sys

sys.path.insert(0, "scripts")
from wordcross import pick_candidates, generate

conn = sqlite3.connect("dictionary.db")

for pool in (2, 4, 6, 8, 10, 15, 20, 30, 50, 100, 300):
    placed_list = []
    for seed in range(50):
        words = pick_candidates(conn, pool_size=pool)
        grid = generate(words, target_count=12, restarts=30, seed=seed)
        placed_list.append(len(grid.placements) if grid else 0)
    avg = sum(placed_list) / len(placed_list)
    ok = sum(1 for p in placed_list if p >= 6)  # decent puzzle: >=6 words
    print(f"pool={pool:>3}  avg placed={avg:5.1f}  >=6-word puzzles: {ok}/50")
