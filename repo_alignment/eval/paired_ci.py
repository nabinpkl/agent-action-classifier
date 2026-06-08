#!/usr/bin/env python3
"""Paired-delta bootstrap CI for the repo-alignment eval (ADR-0014).

Input: a results JSON of per-case violation indicators under two conditions,
the subject agent run with the nudge OFF vs ON (E1), or wording A vs B (E2):

    {"cases": [{"id": "structure-no-helpers", "off": 1, "on": 0}, ...]}

Each value is 1 if the target rule was VIOLATED in that run, else 0. The metric
is the paired per-case delta (off - on): positive means the nudge REDUCED
violations. We bootstrap a 95% CI on the mean delta (futureagi 2026 method) and
gate on it exactly like prompt-regression CI:

  - CI entirely > 0  -> SHIP  (nudge significantly reduces violations)
  - CI straddles 0   -> SELF-RETIRE (no measured effect; ADR-0009 kills it)
  - CI entirely < 0  -> HARMFUL (nudge increases violations; remove)

No numpy: pure stdlib so the eval carries no runtime dependency at v0. The
bootstrap is seeded (default 0) so the verdict is reproducible (ADR-0009
determinism), pass --seed to vary.
"""

import argparse
import json
import random
import sys

ALPHA = 0.05
N_BOOT = 10_000


def paired_delta_ci(deltas, n_boot=N_BOOT, alpha=ALPHA, seed=0):
    """Mean paired delta and a bootstrap (1-alpha) CI. deltas: list of off-on."""
    if not deltas:
        raise ValueError("no cases to analyze")
    rng = random.Random(seed)
    n = len(deltas)
    means = []
    for _ in range(n_boot):
        resample = [deltas[rng.randrange(n)] for _ in range(n)]
        means.append(sum(resample) / n)
    means.sort()
    lo = means[int((alpha / 2) * n_boot)]
    hi = means[int((1 - alpha / 2) * n_boot)]
    return sum(deltas) / n, lo, hi


def verdict(lo, hi):
    if lo > 0:
        return "SHIP"
    if hi < 0:
        return "HARMFUL"
    return "SELF-RETIRE"


def analyze(cases, seed=0):
    deltas = [c["off"] - c["on"] for c in cases]
    off_rate = sum(c["off"] for c in cases) / len(cases)
    on_rate = sum(c["on"] for c in cases) / len(cases)
    mean, lo, hi = paired_delta_ci(deltas, seed=seed)
    return {
        "n": len(cases),
        "violation_rate_off": round(off_rate, 3),
        "violation_rate_on": round(on_rate, 3),
        "mean_delta": round(mean, 3),
        "ci95": [round(lo, 3), round(hi, 3)],
        "verdict": verdict(lo, hi),
    }


def _report(result):
    print(f"  cases (n)           : {result['n']}")
    print(f"  violation rate OFF  : {result['violation_rate_off']}")
    print(f"  violation rate ON   : {result['violation_rate_on']}")
    print(f"  mean paired delta   : {result['mean_delta']}  (off - on; >0 = nudge helps)")
    print(f"  95% CI on delta     : {result['ci95']}")
    print(f"  VERDICT             : {result['verdict']}")


def _self_test():
    # Synthetic: a clearly-helpful nudge (off mostly violates, on mostly clean).
    helpful = [{"id": f"c{i}", "off": 1, "on": 0} for i in range(10)] + [
        {"id": "c10", "off": 0, "on": 0},
        {"id": "c11", "off": 1, "on": 1},
    ]
    r = analyze(helpful)
    assert r["verdict"] == "SHIP", r
    assert r["ci95"][0] > 0, r

    # Synthetic: no effect (off == on everywhere) -> CI is [0,0] -> self-retire.
    noeffect = [{"id": f"c{i}", "off": i % 2, "on": i % 2} for i in range(12)]
    r2 = analyze(noeffect)
    assert r2["verdict"] == "SELF-RETIRE", r2

    # Synthetic: harmful (nudge adds violations).
    harmful = [{"id": f"c{i}", "off": 0, "on": 1} for i in range(10)]
    r3 = analyze(harmful)
    assert r3["verdict"] == "HARMFUL", r3

    print("self-test: OK")
    print("\nhelpful case report:")
    _report(r)
    return 0


def main(argv):
    ap = argparse.ArgumentParser(description="paired-delta bootstrap CI for the adherence eval")
    ap.add_argument("results", nargs="?", help="results JSON ({cases:[{id,off,on}]})")
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args(argv)

    if args.self_test:
        return _self_test()
    if not args.results:
        ap.error("results JSON path required (or --self-test)")

    with open(args.results) as f:
        data = json.load(f)
    result = analyze(data["cases"], seed=args.seed)
    _report(result)
    print()
    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
