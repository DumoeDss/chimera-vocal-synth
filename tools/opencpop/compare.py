"""Sing Opencpop's songs with a CHIMERA build and compare each rhyme with the singer's.

  python tools/opencpop/compare.py <opencpop root> <chimera exe> [--out out/opencpop]
      [--limit n] [--ids a,b,...] [--body female] [--against other/report.json]
      [--her-spans spans.json]   (selftest.py only: where a stand-in singer's finals really are)

The synth sings her phonemes at her times and her score's pitches (bpm 60, a beat a second, each
vowel's beat at the annotated start of its final). What it reports, per final and per initial:
how long she holds each sound and how the synth does, and how each rhyme moves into its ending.
Nothing is trained and nothing from the dataset is written but the numbers in the report.
See docs/opencpop-comparison.md.
"""
import argparse, json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import opencpop as oc

def find(root):
    for sub in ("segments", ""):
        t = os.path.join(root, sub, "transcriptions.txt")
        if os.path.exists(t): return t, os.path.join(root, sub, "wavs")
    raise SystemExit(f"no transcriptions.txt under {root} (expected segments/transcriptions.txt)")

def med(xs): return float(np.median(xs)) if len(xs) else float("nan")
def iqr(xs): return (float(np.percentile(xs, 25)), float(np.percentile(xs, 75))) if len(xs) else (float("nan"),) * 2
def ms(x): return "–" if x != x else f"{1000 * x:.0f}"
def f1(x): return "–" if x != x else f"{x:.1f}"
def kind(final): return "glide" if final in oc.GLIDE else "-n" if final in oc.NASAL_N else "-ng" if final in oc.NASAL_NG else ""

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("root"); ap.add_argument("exe")
    ap.add_argument("--out", default="out/opencpop"); ap.add_argument("--limit", type=int)
    ap.add_argument("--ids"); ap.add_argument("--body", default="female"); ap.add_argument("--against")
    ap.add_argument("--her-spans")
    a = ap.parse_args()
    her_spans = json.load(open(a.her_spans, encoding="utf-8")) if a.her_spans else {}
    trans, wavs = find(a.root)
    os.makedirs(os.path.join(a.out, "renders"), exist_ok=True)
    want = set(a.ids.split(",")) if a.ids else None
    utts, bad_lines = [], 0
    for line in open(trans, encoding="utf-8"):
        if not line.strip(): continue
        try: uid, text, syls = oc.parse_line(line)
        except ValueError: bad_lines += 1; continue
        if want and uid not in want: continue
        if syls: utts.append((uid, syls))
        if a.limit and len(utts) >= a.limit: break
    unread = set(oc.unreadable(a.exe, [s.py for _, syls in utts for s in syls]))

    # ---- her timing, from the annotation alone
    ini, fin = {}, {}
    for _, syls in utts:
        for s in syls:
            if s.initial: ini.setdefault(s.initial, []).append(s.v0 - s.c0)
            if not s.slurred: fin.setdefault(s.final, []).append(s.v1 - s.v0)

    # ---- the renders, measured beside the recordings
    rows, synth_ini, skipped = [], {}, {"unreadable": 0, "no audio": 0, "render failed": 0, "off the beat": 0}
    has_plan = None
    for n, (uid, syls) in enumerate(utts):
        if any(s.py in unread for s in syls): skipped["unreadable"] += 1; continue
        rec = os.path.join(wavs, uid + ".wav")
        if not os.path.exists(rec): skipped["no audio"] += 1; continue
        mel = oc.melody(syls)
        try: wav, segs = oc.render(a.exe, mel, os.path.join(a.out, "renders", uid), a.body)
        except RuntimeError: skipped["render failed"] += 1; continue
        has_plan = segs is not None
        hx, hsr = oc.read_wav(rec); sx, ssr = oc.read_wav(wav)
        her, syn = oc.Track(hx, hsr), oc.Track(sx, ssr)
        placed = None
        if segs is not None:
            front = [x for x in oc.sung(a.exe, mel) if not x.get("melisma")]
            placed = oc.place(segs, front, [oc.PAD + s.v0 for s in syls])
        for k, s in enumerate(syls):
            spot = placed[k] if placed else (oc.PAD + s.c0, oc.PAD + s.v0, oc.PAD + s.v1)
            if spot is None: skipped["off the beat"] += 1; continue
            if s.initial and placed: synth_ini.setdefault(s.initial, []).append(spot[1] - spot[0])
            if s.slurred or not kind(s.final) or s.v1 - s.v0 < 0.15: continue
            hv = her_spans[uid][k] if uid in her_spans else (s.v0, s.v1)
            if hv is None: continue
            h, y = oc.rhyme_measures(her, hv[0], hv[1]), oc.rhyme_measures(syn, spot[1], spot[2])
            if h is None or y is None: continue
            cents = oc.f0_track(her, s.v0, min(s.v1, s.v0 + 0.3), s.parts[0][0])
            rows.append({"id": uid, "py": s.py, "final": s.final, "kind": kind(s.final), "her": h, "synth": y,
                         "her_onset_cents": [float(np.nanmedian(cents[i:j])) if np.isfinite(cents[i:j]).any() else None for i, j in ((0, 3), (3, 6), (6, 10), (10, 20))]})
        print(f"\r{n + 1}/{len(utts)} utterances, {len(rows)} rhymes measured", end="", file=sys.stderr)
    print(file=sys.stderr)

    # ---- the report
    L = []
    L += ["# Opencpop comparison", "", f"`{a.exe}`, body {a.body}; {len(utts)} utterances read, {len(rows)} rhymes measured.", ""]
    L += ["Skipped: " + ", ".join(f"{v} {k}" for k, v in skipped.items()) + (f"; {bad_lines} lines not parsed" if bad_lines else "") + ".", ""]
    if unread: L += ["Syllables the front end doesn't read: " + " ".join(sorted(unread)), ""]

    L += ["## How each rhyme moves into its ending", "",
          "Medians. *held*: how long the ending's sound is held before the final ends; *t50*: when the rhyme is half-way to its ending, before the final's end; *cross*: how long the middle half of the movement takes; *drop*: the last 15% under the loudest part.", "",
          "| final | n | length (ms) | held (ms): her / synth | t50 (ms): her / synth | cross (ms): her / synth | drop (dB): her / synth |", "|---|---|---|---|---|---|---|"]
    table = {}
    groups = [("all off-glides", lambda r: r["kind"] == "glide"), ("all -n", lambda r: r["kind"] == "-n"), ("all -ng", lambda r: r["kind"] == "-ng")]
    finals = sorted({r["final"] for r in rows}, key=lambda f: (kind(f), f))
    for name, pick in groups + [(f, (lambda f: lambda r: r["final"] == f)(f)) for f in finals]:
        rs = [r for r in rows if pick(r)]
        if not rs: continue
        g = lambda who, key: med([r[who][key] for r in rs])
        table[name] = {"n": len(rs), "her": {k: g("her", k) for k in ("dur", "held", "t50", "cross", "drop", "move")}, "synth": {k: g("synth", k) for k in ("dur", "held", "t50", "cross", "drop", "move")}}
        t = table[name]
        L.append(f"| {name} | {len(rs)} | {ms(t['her']['dur'])} | {ms(t['her']['held'])} / {ms(t['synth']['held'])} | {ms(t['her']['t50'])} / {ms(t['synth']['t50'])} | {ms(t['her']['cross'])} / {ms(t['synth']['cross'])} | {f1(t['her']['drop'])} / {f1(t['synth']['drop'])} |")
    L += [""]

    L += ["## How long each initial lasts", "", "From her annotation (the initial's own length) and from the synth's plan (first onset segment to the vowel).", "",
          "| initial | n | her median (ms) | her middle half | synth median (ms) |", "|---|---|---|---|---|"]
    ini_table = {}
    for i in sorted(ini, key=lambda i: -len(ini[i])):
        lo, hi = iqr(ini[i]); sm = med(synth_ini.get(i, []))
        ini_table[i] = {"n": len(ini[i]), "her": med(ini[i]), "her_iqr": [lo, hi], "synth": sm}
        L.append(f"| {i} | {len(ini[i])} | {ms(med(ini[i]))} | {ms(lo)}–{ms(hi)} | {ms(sm)} |")
    if has_plan is False: L += ["", "(This build's `render notes` has no `--plan`, so the synth's lengths are not known.)"]
    L += [""]

    L += ["## How long she holds each final", "", "| final | n | median (ms) | middle half |", "|---|---|---|---|"]
    for f in sorted(fin, key=lambda f: -len(fin[f])):
        lo, hi = iqr(fin[f]); L.append(f"| {f} | {len(fin[f])} | {ms(med(fin[f]))} | {ms(lo)}–{ms(hi)} |")
    L += [""]

    on = np.array([[np.nan if v is None else v for v in r["her_onset_cents"]] for r in rows]) if rows else np.zeros((0, 4))
    if len(on):
        L += ["## Her pitch into a note", "", "Cents from the score's note, medians over the measured rhymes, by time after the final starts.", "",
              "| 0–30 ms | 30–60 ms | 60–100 ms | 100–200 ms |", "|---|---|---|---|",
              "| " + " | ".join("–" if not np.isfinite(on[:, j]).any() else f"{np.nanmedian(on[:, j]):+.0f}" for j in range(4)) + " |", ""]

    report = {"exe": a.exe, "body": a.body, "utterances": len(utts), "rhymes": len(rows), "skipped": skipped, "unreadable": sorted(unread), "finals": table, "initials": ini_table}
    if a.against:
        old = json.load(open(a.against, encoding="utf-8"))
        L += [f"## Against `{a.against}`", "", "The gap to her (|synth − her|, ms) in *held* and *t50*, before → now. Smaller is closer to her.", "", "| final | held | t50 |", "|---|---|---|"]
        for name, t in table.items():
            o = old.get("finals", {}).get(name)
            if not o: continue
            gap = lambda x, k: abs(x["synth"][k] - x["her"][k])
            L.append(f"| {name} | {ms(gap(o, 'held'))} → {ms(gap(t, 'held'))} | {ms(gap(o, 't50'))} → {ms(gap(t, 't50'))} |")
        L += [""]
    open(os.path.join(a.out, "report.md"), "w", encoding="utf-8").write("\n".join(L))
    json.dump(report, open(os.path.join(a.out, "report.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    json.dump(rows, open(os.path.join(a.out, "rhymes.json"), "w", encoding="utf-8"), ensure_ascii=False)
    print(os.path.join(a.out, "report.md"))

if __name__ == "__main__":
    main()
