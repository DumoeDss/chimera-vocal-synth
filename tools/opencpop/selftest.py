"""The comparison run end to end without the dataset: a stand-in "singer" is another CHIMERA build.

  python tools/opencpop/selftest.py <singer exe> <exe under test> [--out out/opencpop-selftest]

Writes a few made-up lines in Opencpop's format, has the singer build sing them as the
"recordings", and runs compare.py on the build under test. The stand-in's finals are where its own
plan put them, not where the made-up annotation says, so its build needs `render notes --plan`
and compare.py is told those spans. With the same build twice the report must show no difference,
and with an older build as the singer it must show where the two differ (the rhymes' endings):
that is the check that the comparison measures what it says.
"""
import argparse, json, os, subprocess, sys, wave
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import opencpop as oc

def line(uid, text, syls):
    """syls: (initial or '', final, note name, initial s, final s[, slur note names...]) or 'AP'."""
    ph, notes, nd, pd, sl = [], [], [], [], []
    for s in syls:
        if s == "AP":
            ph.append("AP"); notes.append("rest"); nd.append(0.3); pd.append(0.3); sl.append(0); continue
        ini, fin, note, di, df, *slurs = s
        if ini: ph.append(ini); notes.append(note); nd.append(di + df); pd.append(di); sl.append(0)
        ph.append(fin); notes.append(note); nd.append(di + df); pd.append(df); sl.append(0)
        for extra in slurs:
            ph.append(fin); notes.append(extra); nd.append(0.3); pd.append(0.3); sl.append(1)
    j = lambda xs: " ".join(str(x) for x in xs)
    return "|".join([uid, text, j(ph), j(notes), j(nd), j(pd), j(sl)])

LINES = [
    line("t0001", "老虎跑得快", [("l", "ao", "C4", 0.06, 0.5), ("h", "u", "D4", 0.08, 0.4), ("p", "ao", "E4", 0.09, 0.5), ("d", "e", "C4", 0.04, 0.3), ("k", "uai", "E4", 0.1, 0.6), "AP"]),
    line("t0002", "两只安昂东风", [("l", "iang", "C4", 0.06, 0.5), ("zh", "i", "D4", 0.07, 0.3), ("", "an", "E4", 0, 0.5), ("", "ang", "D4", 0, 0.5), ("d", "ong", "C4", 0.04, 0.5), ("f", "eng", "E4", 0.1, 0.6), "AP"]),
    line("t0003", "爱你美好", [("", "ai", "E4", 0, 0.5, "D4"), ("n", "i", "C4", 0.07, 0.4), "AP", ("m", "ei", "D4", 0.08, 0.5), ("h", "ao", "C4", 0.08, 0.7), "AP"]),
    line("t0004", "天空有云", [("t", "ian", "G4", 0.09, 0.5), ("k", "ong", "E4", 0.1, 0.5), ("y", "ou", "D4", 0.05, 0.5), ("y", "vn", "C4", 0.05, 0.6), "AP"]),
]

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("singer"); ap.add_argument("exe"); ap.add_argument("--out", default="out/opencpop-selftest")
    a = ap.parse_args()
    root = os.path.join(a.out, "root", "segments"); os.makedirs(os.path.join(root, "wavs"), exist_ok=True)
    os.makedirs(os.path.join(a.out, "singer"), exist_ok=True)
    open(os.path.join(root, "transcriptions.txt"), "w", encoding="utf-8").write("\n".join(LINES) + "\n")
    spans = {}
    for l in LINES:
        uid, _, syls = oc.parse_line(l)
        mel = oc.melody(syls)
        wav, segs = oc.render(a.singer, mel, os.path.join(a.out, "singer", uid))
        if segs is None: raise SystemExit("the singer build's `render notes` has no --plan")
        front = [x for x in oc.sung(a.singer, mel) if not x.get("melisma")]
        spans[uid] = [None if p is None else [p[1] - oc.PAD, p[3] - oc.PAD] for p in oc.place(segs, front, [oc.PAD + s.v0 for s in syls])]
        x, sr = oc.read_wav(wav)
        x = x[int(oc.PAD * sr):]  # the recording's time starts where the annotation's does
        w = wave.open(os.path.join(root, "wavs", uid + ".wav"), "wb"); w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr)
        w.writeframes((np.clip(x, -1, 1) * 32767).astype(np.int16).tobytes()); w.close()
    here = os.path.dirname(os.path.abspath(__file__))
    json.dump(spans, open(os.path.join(a.out, "her-spans.json"), "w", encoding="utf-8"))
    r = subprocess.run([sys.executable, "-X", "utf8", os.path.join(here, "compare.py"), os.path.join(a.out, "root"), a.exe, "--out", os.path.join(a.out, "report"), "--her-spans", os.path.join(a.out, "her-spans.json")])
    sys.exit(r.returncode)

if __name__ == "__main__":
    main()
