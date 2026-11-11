#!/usr/bin/env python3
"""The Import page from FAKE config files of every source the core knows (fake_home.py, in the instance's scratch HOME):
each card opened by pointer, its review rows listed, one row unticked, Import selected pressed, what config.json took."""
import sys
sys.argv = ["x", sys.argv[1] if len(sys.argv) > 1 else "b"]
exec(open(__file__.replace("imports.py", "lib.py")).read())

def snap():
    c = config()
    return {k: v for k, v in c.items() if k != "feature_flags"}

say("open Import", "settings-section import", "settings-size 920x1400", "pointer-window Settings")
time.sleep(2)
cards = [r for r in rows() if re.fullmatch(r"import\.[a-z0-9_-]+", r["id"]) and r["id"] != "import.apply"]
print("cards found:", [r["id"] for r in cards], flush=True)
for card in cards:
    sid = card["id"].split(".", 1)[1]
    print(f"\n=== {sid}", flush=True)
    before = snap()
    click_id(card["id"]); time.sleep(1.2)
    review = [r["id"] for r in rows() if r["id"].startswith(f"import.{sid}.tick")]
    print("   rows to tick:", review, flush=True)
    shot(f"imp-{sid}")
    if len(review) > 1:  # untick the last one: it must not be applied
        click_id(review[-1]); time.sleep(0.4)
        print("   unticked", review[-1], flush=True)
    click_id("import.apply"); time.sleep(1.5)
    after = snap()
    changed = {k: (before.get(k), after[k]) for k in after if before.get(k) != after[k]}
    print("   config.json took:", json.dumps(changed)[:400], flush=True)
    click_id(card["id"]); time.sleep(0.8)   # close the card
