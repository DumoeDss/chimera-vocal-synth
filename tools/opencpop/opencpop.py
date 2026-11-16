"""A Mandarin singing corpus's annotation, read into sung syllables on the recording's timeline,
and the signal measures the comparison uses. See docs/opencpop-comparison.md.

Two corpora, the same idea: one entry per phoneme (a pinyin initial or final, or a rest), each with
its length, its note and a slur flag; a slurred entry repeats the final on a further note (a
melisma).
  Opencpop   segments/transcriptions.txt, a line: id|text|phonemes|notes|note durations|phoneme
             durations|slur flags. Notes are names (E4, F#4/Gb4, rest); y and w are initials.
  M4Singer   meta.json, a list of {item_name, phs, is_slur, ph_dur, notes, notes_dur}. Notes are
             MIDI numbers (0: a rest); a syllable with no initial is its final alone (uo for wo).
"""
import json, math, os, subprocess, wave
import numpy as np

INITIALS = {"b", "p", "m", "f", "d", "t", "n", "l", "g", "k", "h", "j", "q", "x", "zh", "ch", "sh", "r", "z", "c", "s", "y", "w"}
RESTS = {"SP", "AP", "<SP>", "<AP>"}
PAD = 0.3  # the render's lead: its time is the recording's plus this

# finals whose ending is an off-glide or a nasal: where a rhyme can be one movement or two sounds
GLIDE = {"ai", "ei", "ao", "ou", "iao", "iou", "iu", "uai", "uei", "ui"}
NASAL_N = {"an", "en", "in", "ian", "uan", "uen", "un", "van", "vn"}
NASAL_NG = {"ang", "eng", "ing", "ong", "iang", "uang", "ueng", "iong"}

NOTE = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}
def midi(name):
    """'E4', 'F#4/Gb4', 'rest' -> a MIDI number, or 0 for a rest."""
    name = name.split("/")[0]
    if name.lower() == "rest": return 0
    step = NOTE[name[0]]
    acc = name.count("#") - name.count("b")
    return 12 * (int(name.lstrip("ABCDEFG#b")) + 1) + step + acc

ZERO = {"i": "yi", "in": "yin", "ing": "ying", "iou": "you", "iu": "you", "u": "wu", "uei": "wei", "ui": "wei",
        "uen": "wen", "un": "wen", "ueng": "weng", "v": "yu", "ve": "yue", "van": "yuan", "vn": "yun"}
def pinyin(initial, final):
    """A corpus's initial and final -> the pinyin syllable the Pinyin front end reads (toneless)."""
    f = final
    if not initial:  # a final alone is spelled with y or w: iang -> yang, uo -> wo, v -> yu
        if f in ZERO: return ZERO[f]
        if f[0] == "i": return "y" + f[1:]
        if f[0] == "u": return "w" + f[1:]
        return f
    if initial in ("j", "q", "x", "y") and f.startswith("v"): f = "u" + f[1:]
    if initial not in ("y", "w"):
        f = {"iou": "iu", "uei": "ui", "uen": "un"}.get(f, f)
    return initial + f

# The Pinyin front end leaves a one-letter word to English ("a" is a word), so the three syllables
# that are one letter are sung as a character each (the Han front end; its lexicon is found from
# the repository's root, where this is run).
LONE = {"a": "啊", "o": "哦", "e": "饿"}
def word(py):
    """A toneless pinyin syllable as a lyric word the front ends read as Mandarin."""
    return LONE.get(py, py + "1")

class Syllable:
    __slots__ = ("initial", "final", "py", "c0", "v0", "v1", "parts")
    def __init__(self, initial, final, c0, v0, v1, pitch):
        self.initial, self.final, self.py = initial, final, pinyin(initial, final)
        self.c0, self.v0, self.v1 = c0, v0, v1          # consonant start, final start, final end (s)
        self.parts = [(pitch, v0, v1)]                  # (midi, start, end): the final's note, then slurs
    @property
    def end(self): return self.parts[-1][2]
    @property
    def slurred(self): return len(self.parts) > 1

def parse_line(line):
    """An Opencpop line -> (id, text, [Syllable]); times start at 0 at the segment's start."""
    f = line.rstrip("\n").split("|")
    if len(f) != 7: raise ValueError(f"expected 7 fields, got {len(f)}")
    return f[0], f[1], syllables(f[0], f[2].split(), [midi(x) for x in f[3].split()], [float(x) for x in f[5].split()], [int(x) for x in f[6].split()])

def syllables(uid, ph, notes, dur, slur):
    """One entry per phoneme (name, MIDI note or 0, length in s, slur flag) -> [Syllable]."""
    if not (len(ph) == len(notes) == len(dur) == len(slur)): raise ValueError(f"{uid}: field lengths differ")
    t, out, i = 0.0, [], 0
    while i < len(ph):
        p = ph[i]
        if p in RESTS:
            t += dur[i]; i += 1
        elif slur[i] == 1:
            if not out: raise ValueError(f"{uid}: a slur with no syllable before it")
            out[-1].parts.append((notes[i], t, t + dur[i])); t += dur[i]; i += 1
        elif p in INITIALS and i + 1 < len(ph) and ph[i + 1] not in RESTS and ph[i + 1] not in INITIALS and slur[i + 1] == 0:
            out.append(Syllable(p, ph[i + 1], t, t + dur[i], t + dur[i] + dur[i + 1], notes[i + 1]))
            t += dur[i] + dur[i + 1]; i += 2
        else:
            out.append(Syllable("", p, t, t, t + dur[i], notes[i])); t += dur[i]; i += 1
    return [s for s in out if all(m > 0 for m, _, _ in s.parts)]  # a syllable on a rest has no pitch to sing

def load(root):
    """A corpus folder -> [(id, wav path, voice, [Syllable])] and the lines that didn't parse.
    voice: "female" or "male", the body that sings it (M4Singer's tenors and basses are male)."""
    out, bad = [], 0
    meta = os.path.join(root, "meta.json")
    if os.path.exists(meta):
        for e in json.load(open(meta, encoding="utf-8")):
            singer, song, idx = e["item_name"].split("#")
            try: syls = syllables(e["item_name"], e["phs"], e["notes"], e["ph_dur"], e["is_slur"])
            except ValueError: bad += 1; continue
            voice = "male" if singer.startswith(("Tenor", "Bass")) else "female"
            if syls: out.append((e["item_name"], os.path.join(root, f"{singer}#{song}", idx + ".wav"), voice, syls))
        return out, bad
    for sub in ("segments", ""):
        t = os.path.join(root, sub, "transcriptions.txt")
        if not os.path.exists(t): continue
        for line in open(t, encoding="utf-8"):
            if not line.strip(): continue
            try: uid, _, syls = parse_line(line)
            except ValueError: bad += 1; continue
            if syls: out.append((uid, os.path.join(root, sub, "wavs", uid + ".wav"), "female", syls))
        return out, bad
    raise SystemExit(f"neither meta.json (M4Singer) nor segments/transcriptions.txt (Opencpop) under {root}")

def melody(syls, contiguous=0.03):
    """The syllables as a `chimera render notes` melody on the recording's timeline: each vowel's
    beat at the final's annotated start, held to the next vowel when the next syllable follows at
    once (its consonant takes its time from this note, as in the planner), else to its own end and
    a rest. One second a beat."""
    notes, words = [], []
    for k, s in enumerate(syls):
        nxt = syls[k + 1] if k + 1 < len(syls) else None
        joined = nxt is not None and nxt.c0 - s.end < contiguous
        for j, (m, a, b) in enumerate(s.parts):
            if j == len(s.parts) - 1 and joined: b = nxt.v0
            notes.append([m, round(b - a, 5)])
            words.append(word(s.py) if j == 0 else "~")
        if nxt is not None and not joined:
            notes.append([0, round(nxt.v0 - s.end, 5)])
    return {"bpm": 60, "t0": round(PAD + syls[0].v0, 5), "lyrics": " ".join(words), "notes": notes}

def unreadable(exe, pys):
    """The syllables the build's front end does not read as pinyin."""
    def bad(batch):
        r = subprocess.run([exe, "syllables", " ".join(word(p) for p in batch)], capture_output=True, text=True, encoding="utf-8")
        try: got = [s["text"] for s in json.loads(r.stdout)]
        except Exception: got = []
        if got == [word(p) for p in batch]: return []
        return batch if len(batch) == 1 else bad(batch[:len(batch) // 2]) + bad(batch[len(batch) // 2:])
    pys = sorted(set(pys))
    return bad(pys) if pys else []

def render(exe, mel, out_stem, body="female"):
    """Sing a melody with a build's CLI. -> (the WAV's path, the planned segments, or None from a
    build whose `render notes` has no --plan)."""
    path, wav, plan = out_stem + ".json", out_stem + ".wav", out_stem + ".plan.json"
    json.dump(mel, open(path, "w", encoding="utf-8"), ensure_ascii=False)
    for extra in (["--plan", plan], []):
        if os.path.exists(plan): os.remove(plan)
        r = subprocess.run([exe, "render", "notes", path, out_stem, body] + extra, capture_output=True, text=True, encoding="utf-8")
        if r.returncode == 0 and os.path.exists(wav):
            segs = json.load(open(plan, encoding="utf-8"))["segments"] if extra and os.path.exists(plan) else None
            return wav, segs
    raise RuntimeError(f"render failed: {(r.stderr or r.stdout)[:300]}")

def sung(exe, mel):
    """The front end's syllables for a melody's lyrics, one per sounding note."""
    r = subprocess.run([exe, "syllables", mel["lyrics"]], capture_output=True, text=True, encoding="utf-8")
    return json.loads(r.stdout)

def place(segs, syls, beats, tol=0.03):
    """Where the plan put each syllable: for syllable k, (onset start, vowel start, rhyme end) on the
    render's timeline, or None when its vowel is more than `tol` from its beat (a note too short
    for its consonants pushes the vowels late) or it is a melisma's continuation.

    The onset is the syllable's consonants and, for a rising nucleus, the short glide that ends on
    the beat. The rhyme is the vowels from the beat on (an off-glide, a melisma's further notes)
    and then the syllable's coda, never past the next syllable's onset."""
    spots, i = [], 0
    for syl, beat in zip(syls, beats):
        if syl.get("melisma"): spots.append(None); continue
        while i < len(segs) and not (segs[i]["kind"] == "vowel" and segs[i]["t0"] >= beat - tol): i += 1
        if i >= len(segs) or abs(segs[i]["t0"] - beat) > tol: spots.append(None); continue
        v = a = i
        if a > 0 and segs[a - 1]["kind"] == "vowel" and abs(segs[a - 1]["t1"] - segs[v]["t0"]) < 1e-6 and segs[a - 1]["t1"] - segs[a - 1]["t0"] < 0.08: a -= 1
        a = max(0, a - len(syl.get("onset", [])))
        spots.append((a, v, len(syl.get("coda", []))))
        i = v + 1
    joined = lambda e: abs(segs[e + 1]["t0"] - segs[e]["t1"]) < 1e-6
    out = []
    for k, sp in enumerate(spots):
        if sp is None: out.append(None); continue
        a, v, n_coda = sp
        limit = next((x[0] for x in spots[k + 1:] if x is not None), len(segs))
        e = v
        while e + 1 < limit and joined(e) and segs[e + 1]["kind"] == "vowel": e += 1
        c = 0
        while c < n_coda and e + 1 < limit and joined(e) and segs[e + 1]["kind"] != "vowel": e += 1; c += 1
        out.append((segs[a]["t0"], segs[v]["t0"], segs[e]["t1"]))
    return out

def read_wav(path):
    w = wave.open(path); sr, n, ch, sw = w.getframerate(), w.getnframes(), w.getnchannels(), w.getsampwidth()
    raw = w.readframes(n)
    if sw == 2: x = np.frombuffer(raw, dtype=np.int16).astype(np.float64) / 32768
    elif sw == 3:
        b = np.frombuffer(raw, dtype=np.uint8).reshape(-1, 3).astype(np.int32)
        x = ((b[:, 0] | (b[:, 1] << 8) | (b[:, 2] << 16)) - ((b[:, 2] >> 7) << 24)).astype(np.float64) / 8388608
    elif sw == 4: x = np.frombuffer(raw, dtype=np.int32).astype(np.float64) / 2147483648
    else: raise ValueError(f"{path}: {8 * sw}-bit audio")
    if ch > 1: x = x.reshape(-1, ch).mean(axis=1)
    return x, sr

# ---- signal measures: 25 ms frames every 5 ms
HOP, WIN, N_MEL, N_CEP, FMAX = 0.005, 0.025, 40, 13, 8000.0

def _mel_bank(sr, nfft):
    mel = lambda f: 2595 * np.log10(1 + f / 700)
    inv = lambda m: 700 * (10 ** (m / 2595) - 1)
    edges = inv(np.linspace(mel(60.0), mel(min(FMAX, sr / 2)), N_MEL + 2))
    freqs = np.fft.rfftfreq(nfft, 1 / sr)
    bank = np.zeros((N_MEL, len(freqs)))
    for i in range(N_MEL):
        lo, mid, hi = edges[i:i + 3]
        bank[i] = np.clip(np.minimum((freqs - lo) / (mid - lo), (hi - freqs) / (hi - mid)), 0, None)
    return bank

class Track:
    """A signal's frames: mel cepstra (without the level), level in dB, and the frame times."""
    def __init__(self, x, sr):
        n, hop = int(WIN * sr), int(HOP * sr)
        nfft = 1 << (n - 1).bit_length()
        count = max(0, 1 + (len(x) - n) // hop)
        idx = np.arange(n)[None, :] + hop * np.arange(count)[:, None]
        frames = x[idx] * np.hanning(n)[None, :]
        power = np.abs(np.fft.rfft(frames, nfft, axis=1)) ** 2
        logmel = np.log(power @ _mel_bank(sr, nfft).T + 1e-12)
        k = np.arange(N_MEL)
        dct = np.cos(np.pi * np.outer(np.arange(1, N_CEP), k + 0.5) / N_MEL)
        self.cep = logmel @ dct.T
        self.db = 10 * np.log10(np.mean(frames ** 2, axis=1) + 1e-12)
        self.t = (np.arange(count) * hop + n / 2) / sr
        self.x, self.sr = x, sr
    def span(self, a, b):
        i, j = np.searchsorted(self.t, a), np.searchsorted(self.t, b)
        return slice(i, max(i, j))

def _smooth(v, n=5):
    if len(v) < n: return v
    pad = np.pad(v, (n // 2, n // 2), mode="edge")
    return np.median(np.lib.stride_tricks.sliding_window_view(pad, n), axis=-1)

def rhyme_measures(trk, a, b):
    """How a final from a to b (s) moves from the sound it starts with to the sound it ends with.

    The position p(t) of each frame is its place on the line between the final's start (the mean
    cepstrum of its first fifth) and its end (the mean of its last 15%): 0 at the start's sound, 1
    at the end's. A voice that holds one sound and jumps to another crosses fast and then sits at 1;
    one movement into the ending crosses late and arrives only at the end.
      move   how far apart the two sounds are (cepstral distance): small means nothing moved
      t50    when p crosses 0.5, as seconds before the final's end
      cross  how long the crossing from 0.25 to 0.75 takes (s)
      held   how long the ending is held: the time from p first reaching 0.9 to the end (s)
      drop   the level of the last 15% under the loudest part of the final (dB)
    None when the final is too short to tell (under 20 frames)."""
    sl = trk.span(a + 0.01, b - 0.005)
    c, db, t = trk.cep[sl], trk.db[sl], trk.t[sl]
    n = len(c)
    if n < 20: return None
    s, e = c[:max(2, n // 5)].mean(axis=0), c[-max(2, int(0.15 * n)):].mean(axis=0)
    d = e - s; move = float(np.sqrt(d @ d))
    p = _smooth(np.clip((c - s) @ d / max(d @ d, 1e-9), -0.5, 1.5))
    def first(level):
        hit = np.nonzero(p >= level)[0]
        return t[hit[0]] if len(hit) else t[-1]
    end = b
    top = np.sort(db)[-max(2, n // 5):].mean()
    return {"dur": b - a, "move": move, "t50": end - first(0.5), "cross": max(0.0, first(0.75) - first(0.25)),
            "held": end - first(0.9), "drop": float(top - db[-max(2, int(0.15 * n)):].mean())}

def f0_track(trk, a, b, note_midi, span_st=7):
    """The pitch (cents from the note) of the frames from a to b, searched within `span_st` semitones
    of the note so it can't jump an octave; NaN where there is no clear period."""
    f_note = 440 * 2 ** ((note_midi - 69) / 12)
    sr, x = trk.sr, trk.x
    lo, hi = int(sr / (f_note * 2 ** (span_st / 12))), int(sr / (f_note / 2 ** (span_st / 12))) + 2
    n = max(int(0.04 * sr), 3 * hi)
    out = []
    for t in np.arange(a, b, 0.01):
        i = int(t * sr) - n // 2
        if i < 0 or i + n > len(x): out.append(np.nan); continue
        seg = x[i:i + n] - x[i:i + n].mean()
        ac = np.array([seg[:n - hi] @ seg[k:k + n - hi] for k in range(lo, hi + 1)])
        e0 = seg[:n - hi] @ seg[:n - hi]
        k = int(np.argmax(ac))
        if e0 <= 0 or ac[k] < 0.5 * e0: out.append(np.nan); continue
        k2 = k
        if 0 < k < len(ac) - 1:
            y0, y1, y2 = ac[k - 1], ac[k], ac[k + 1]
            den = y0 - 2 * y1 + y2
            if den != 0: k2 = k + 0.5 * (y0 - y2) / den
        out.append(1200 * math.log2(sr / (lo + k2) / f_note))
    return np.array(out)
