"""Pool throughput with a random policy answered from Python: decisions and
games per second, per batch size (`uv run python -m train.bench_pool`)."""

import sys
import time

import numpy as np
import zvtes_py


def bench(n_games, seconds=5.0):
    pool = zvtes_py.GamePool(n_games, ["policy"] * 4, seed=0)
    rng = np.random.default_rng(0)
    decisions = games = 0
    observe = 0.0
    start = time.perf_counter()
    while time.perf_counter() - start < seconds:
        t = time.perf_counter()
        obs = pool.observe()
        observe += time.perf_counter() - t
        actions = (rng.random(n_games) * obs["opt_n"]).astype(np.int64).tolist()
        games += len(pool.act(actions))
        decisions += n_games
    wall = time.perf_counter() - start
    print(
        f"{n_games:5d} games: {decisions / wall:9.0f} decisions/s, "
        f"{games / wall:6.1f} games/s, observe {100 * observe / wall:.0f}% of wall"
    )


if __name__ == "__main__":
    for n in [int(a) for a in sys.argv[1:]] or [64, 256, 1024]:
        bench(n)
