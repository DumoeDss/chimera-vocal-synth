"""Sing a Mandarin singing corpus's songs with a CHIMERA build and compare each rhyme with the singer's.

  python tools/opencpop/compare.py <corpus root> <chimera exe> [--out out/opencpop]
      [--singers a,b] [--per-singer n | --blocks n | --same-as other-run] [--limit n] [--ids a,b,...]
      [--jobs n] [--against other/report.json]
      [--her-spans spans.json]   (selftest.py only: where a stand-in singer's finals really are)

The corpus is Opencpop or M4Singer (opencpop.py reads both). The synth sings the singer's phonemes
at her times and her score's pitches (bpm 60, a beat a second, each vowel's beat at the annotated
start of its final), on the female body for a woman and the male body for a man. What it reports,
per final and per initial: how long she holds each sound and how the synth does, and how each rhyme
moves into its ending. Nothing is trained and nothing from the corpus is written but the numbers
in the report. See docs/opencpop-comparison.md.
"""
import argparse, concurrent.futures as cf, json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import opencpop as oc

def med(xs): return float(np.median(xs)) if len(xs) else float("nan")
def iqr(xs): return (float(np.percentile(xs, 25)), float(np.percentile(xs, 75))) if len(xs) else (float("nan"),) * 2
def ms(x): return "–" if x != x else f"{1000 * x:.0f}"
def f1(x): return "–" if x != x else f"{x:.1f}"
def kind(final): return "glide" if final in oc.GLIDE else "-n" if final in oc.NASAL_N else "-ng" if final in oc.NASAL_NG else ""
KEYS = ("dur", "held", "t50", "cross", "drop", "move")

def one(job):
    """One utterance: render it, measure its rhymes beside the recording.
    -> (skip reason or None, rhyme rows, {initial: [synth onset lengths]}, had a plan)"""
    n, uid, rec, voice, syls, exe, out, her_span = job
    if not os.path.exists(rec): return "no audio", [], {}, None
    mel = oc.melody(syls)
    try: wav, segs = oc.render(exe, mel, os.path.join(out, "renders", f"{n:06d}"), voice)
    except RuntimeError: return "render failed", [], {}, None
    hx, hsr = oc.read_wav(rec); sx, ssr = oc.read_wav(wav)
    her, syn = oc.Track(hx, hsr), oc.Track(sx, ssr)
    placed = None
    if segs is not None:
        front = [x for x in oc.sung(exe, mel) if not x.get("melisma")]
        placed = oc.place(segs, front, [oc.PAD + s.v0 for s in syls])
    rows, ini, off = [], {}, 0
    for k, s in enumerate(syls):
        spot = placed[k] if placed else (oc.PAD + s.c0, oc.PAD + s.v0, oc.PAD + s.v0, oc.PAD + s.v1)
        if spot is None: off += 1; continue
        # the initial's own length: up to the medial, except Opencpop's y and w, which are the medial
        if s.initial and placed: ini.setdefault(s.initial, []).append(spot[2 if s.initial in ("y", "w") else 1] - spot[0])
        if s.slurred or not kind(s.final) or s.v1 - s.v0 < 0.15: continue
        hv = her_span[k] if her_span is not None else (s.v0, s.v1)
        if hv is None: continue
        h, y = oc.rhyme_measures(her, hv[0], hv[1]), oc.rhyme_measures(syn, spot[1], spot[3])
        if h is None or y is None: continue
        cents = oc.f0_track(her, s.v0, min(s.v1, s.v0 + 0.3), s.parts[0][0])
        onset = [float(np.nanmedian(cents[i:j])) if np.isfinite(cents[i:j]).any() else None for i, j in ((0, 3), (3, 6), (6, 10), (10, 20))]
        rows.append({"id": uid, "voice": voice, "py": s.py, "final": s.final, "kind": kind(s.final), "her": h, "synth": y, "her_onset_cents": onset})
    return None, rows, ini, segs is not None, off

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("root"); ap.add_argument("exe")
    ap.add_argument("--out", default="out/opencpop"); ap.add_argument("--limit", type=int); ap.add_argument("--per-singer", type=int)
    ap.add_argument("--blocks", type=int); ap.add_argument("--singers"); ap.add_argument("--same-as")
    ap.add_argument("--ids"); ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--against"); ap.add_argument("--her-spans")
    a = ap.parse_args()
    her_spans = json.load(open(a.her_spans, encoding="utf-8")) if a.her_spans else {}
    os.makedirs(os.path.join(a.out, "renders"), exist_ok=True)
    everything, bad_lines = oc.load(a.root)

    # ---- her timing, from the annotation alone: every utterance of the corpus
    ini, fin = {}, {}
    for _, _, _, syls in everything:
        for s in syls:
            if s.initial: ini.setdefault(s.initial, []).append(s.v0 - s.c0)
            if not s.slurred: fin.setdefault(s.final, []).append(s.v1 - s.v0)

    # ---- the utterances to sing: all, or the same share of every singer, spread over her songs
    utts = everything
    if a.ids:
        want = set(a.ids.split(",")); utts = [u for u in utts if u[0] in want]
    if a.singers:
        want = set(a.singers.split(",")); utts = [u for u in utts if oc.singer_of(u[0]) in want]
    if a.same_as:
        # the utterances another run sang, in its order: two builds are compared on the same ones
        index = {u[0]: u for u in utts}
        utts = [index[uid] for uid in json.load(open(os.path.join(a.same_as, "utterances.json"), encoding="utf-8")) if uid in index]
    elif a.per_singer:
        utts = oc.sample(utts, a.per_singer)
    if a.blocks:
        # ACE-Opencpop keeps its audio in 40 MB row groups of 100 segments: take whole groups,
        # spread over the files (so over the voices and the songs), every second segment of each
        groups = {}
        for u in utts:
            f, i = oc.SOURCES.get(u[0], ("", 0))
            groups.setdefault((f, i // 100), []).append(u)
        keys = sorted(groups)
        pick = [keys[(j * len(keys)) // a.blocks + (j * 5) % max(1, len(keys) // a.blocks)] for j in range(a.blocks)]
        utts = [u for k in dict.fromkeys(pick) for u in groups[k][::2]]
    if a.limit: utts = utts[:a.limit]
    json.dump([u[0] for u in utts], open(os.path.join(a.out, "utterances.json"), "w", encoding="utf-8"), ensure_ascii=False)
    oc.fetch_audio(utts)
    unread = set(oc.unreadable(a.exe, [s.py for _, _, _, syls in utts for s in syls]))
    skipped = {"unreadable": 0, "no audio": 0, "render failed": 0}
    jobs = []
    for n, (uid, rec, voice, syls) in enumerate(utts):
        if any(s.py in unread for s in syls): skipped["unreadable"] += 1; continue
        jobs.append((n, uid, rec, voice, syls, a.exe, a.out, her_spans.get(uid) if her_spans else None))

    rows, synth_ini, off_beat, has_plan, done = [], {}, 0, None, 0
    with cf.ProcessPoolExecutor(a.jobs) as pool:
        for why, r, i, plan, off in pool.map(one_safe, jobs, chunksize=1):
            done += 1
            if why: skipped[why] += 1
            rows += r; off_beat += off
            for k, v in i.items(): synth_ini.setdefault(k, []).extend(v)
            if plan is not None: has_plan = plan
            if done % 20 == 0 or done == len(jobs): print(f"\r{done}/{len(jobs)} utterances, {len(rows)} rhymes measured", end="", file=sys.stderr)
    print(file=sys.stderr)

    # ---- the report
    L = ["# Comparison with a Mandarin singing corpus", "",
         f"`{a.exe}` against `{a.root}`: {len(everything)} utterances in the corpus, {len(jobs)} sung, {len(rows)} rhymes measured.", "",
         "Skipped: " + ", ".join(f"{v} {k}" for k, v in skipped.items()) + f"; {off_beat} syllables whose vowel the planner put off its beat" + (f"; {bad_lines} entries not parsed" if bad_lines else "") + ".", ""]
    if unread: L += ["Syllables the front end doesn't read: " + " ".join(sorted(unread)), ""]

    L += ["## How each rhyme moves into its ending", "",
          "Medians. *held*: how long the ending's sound is held before the final ends; *t50*: when the rhyme is half-way to its ending, before the final's end; *cross*: how long the middle half of the movement takes; *drop*: the last 15% under the loudest part.", "",
          "| final | n | length (ms) | held (ms): her / synth | t50 (ms): her / synth | cross (ms): her / synth | drop (dB): her / synth |", "|---|---|---|---|---|---|---|"]
    table = {}
    groups = [(f"all {name}{who}", (lambda kd, v: lambda r: r["kind"] == kd and (v is None or r["voice"] == v))(kd, v))
              for name, kd in (("off-glides", "glide"), ("-n", "-n"), ("-ng", "-ng")) for who, v in (("", None), (", women", "female"), (", men", "male"))]
    finals = sorted({r["final"] for r in rows}, key=lambda f: (kind(f), f))
    for name, pick in groups + [(f, (lambda f: lambda r: r["final"] == f)(f)) for f in finals]:
        rs = [r for r in rows if pick(r)]
        if not rs: continue
        t = table[name] = {"n": len(rs), "her": {k: med([r["her"][k] for r in rs]) for k in KEYS}, "synth": {k: med([r["synth"][k] for r in rs]) for k in KEYS}}
        L.append(f"| {name} | {len(rs)} | {ms(t['her']['dur'])} | {ms(t['her']['held'])} / {ms(t['synth']['held'])} | {ms(t['her']['t50'])} / {ms(t['synth']['t50'])} | {ms(t['her']['cross'])} / {ms(t['synth']['cross'])} | {f1(t['her']['drop'])} / {f1(t['synth']['drop'])} |")
    L += [""]

    L += ["## How long each initial lasts", "", "From the annotation of the whole corpus (the initial's own length) and from the synth's plan (first onset segment to the vowel) in the utterances sung.", "",
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

    report = {"exe": a.exe, "root": a.root, "utterances": len(jobs), "rhymes": len(rows), "skipped": skipped, "off_beat": off_beat, "unreadable": sorted(unread), "finals": table, "initials": ini_table}
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

def one_safe(job):
    """`one`, with a failure inside one utterance counted rather than ending the run."""
    try:
        r = one(job)
        return (r[0], r[1], r[2], r[3], r[4]) if len(r) == 5 else (r[0], r[1], r[2], r[3], 0)
    except Exception:
        return "render failed", [], {}, None, 0

if __name__ == "__main__":
    main()
