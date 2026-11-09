"""The scramble test: how intelligible a language front end's singing is on sound alone.

A recogniser that knows a lyric fills it in, so the test that counts sings the same melody with the
lyric's units shuffled (syllables for Chinese and Korean, words for English): nothing to guess from.
Each language's text goes through CHIMERA's own front end (`chimera render notes` with "lyrics"),
on one generated tune, one note per syllable, a breath between lines, in the male and female voices.
SenseVoice (sherpa-onnx, CTC: it can't hallucinate a loop) transcribes one sung line at a time.
Score: syllable errors (Chinese: toneless pinyin; Korean: Hangul blocks) or word errors (English).

usage: scramble.py <outdir> [--langs zh,ko,en] [--shuffles 3] [--bodies male,female]
                   [--text lang:file]   (your own lines for one language, one line per phrase)
Needs the release CLI (cargo build --release -p chimera-cli) and tools/asr/setup.sh.
The texts are public domain: two Tang poems (静夜思, 春晓), the folk song 아리랑, Harvard sentences.
"""

import json
import os
import random
import re
import subprocess
import sys
import wave

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
CLI = os.environ.get("CHIMERA_CLI", os.path.join(REPO, "target", "release", "chimera"))
MODEL = os.environ.get("SENSEVOICE_DIR", os.path.expanduser("~/.cache/chimera-asr/models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17"))

TEXTS = {
    "zh": ["床前明月光", "疑是地上霜", "举头望明月", "低头思故乡", "春眠不觉晓", "处处闻啼鸟", "夜来风雨声", "花落知多少"],
    "ko": ["아리랑 아리랑 아라리요", "아리랑 고개로 넘어간다", "나를 버리고 가시는 님은", "십리도 못 가서 발병 난다"],
    "en": ["The birch canoe slid on the smooth planks", "Glue the sheet to the dark blue background", "It's easy to tell the depth of a well", "These days a chicken leg is a rare dish", "Rice is often served in round bowls"],
}
BPM, T0 = 100.0, 0.3
TUNE = [0, 2, 4, 7, 9, 7, 4, 2, 0, -3, 0, 2]  # a pentatonic wander, from the voice's base note
BASE = {"male": 50, "female": 62}


def units(lang, line):
    """The units the scramble shuffles: syllables (one per character) or words."""
    return line.split() if lang == "en" else [c for c in line if not c.isspace()]


def join(lang, us):
    return " ".join(us) if lang == "en" else "".join(us)


def scramble(lang, lines, seed):
    pool = [u for ln in lines for u in units(lang, ln)]
    random.Random(seed).shuffle(pool)
    out, k = [], 0
    for ln in lines:
        n = len(units(lang, ln))
        out.append(join(lang, pool[k:k + n]))
        k += n
    return out


def syllable_count(line):
    out = subprocess.run([CLI, "syllables", line], capture_output=True, text=True, check=True).stdout
    return sum(1 for s in json.loads(out) if not s.get("melisma"))


def render(lines, body, stem):
    """Render the lines on the tune; return each line's (start, end) in seconds."""
    beat = 60 / BPM
    notes, spans, t, k = [], [], T0, 0
    for ln in lines:
        n = syllable_count(ln)
        start = t
        for _ in range(n):
            notes.append([BASE[body] + TUNE[k % len(TUNE)], 1])
            k += 1
            t += beat
        spans.append((max(0.0, start - 0.3), t + 0.15))
        notes.append([0, 1])  # a breath between lines
        t += beat
    melody = {"bpm": BPM, "t0": T0, "lyrics": "\n".join(lines), "notes": notes}
    with open(stem + ".json", "w") as f:
        json.dump(melody, f, ensure_ascii=False)
    subprocess.run([CLI, "render", "notes", stem + ".json", stem, body], check=True, capture_output=True)
    return spans


def load(path):
    w = wave.open(path)
    x = np.frombuffer(w.readframes(w.getnframes()), "<i2").astype(np.float32) / 32768
    sr = w.getframerate()
    # 48 kHz → 16 kHz (the renders are band-limited well below 8 kHz)
    return x[:: sr // 16000].copy() if sr % 16000 == 0 else x


def edits(r, h):
    d = list(range(len(h) + 1))
    for i, x in enumerate(r, 1):
        prev, d[0] = d[0], i
        for j, y in enumerate(h, 1):
            prev, d[j] = d[j], min(d[j] + 1, d[j - 1] + 1, prev + (x != y))
    return d[len(h)]


def scorer(lang):
    """Reference and hypothesis → comparable unit lists."""
    if lang == "zh":
        from pypinyin import lazy_pinyin
        from zhconv import convert
        return lambda s: lazy_pinyin(re.sub(r"[^一-鿿]", "", convert(s, "zh-cn")))
    if lang == "ko":
        return lambda s: re.findall(r"[가-힣]", s)
    return lambda s: re.sub(r"[^a-z' ]+", " ", s.lower()).split()


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    out = args[0]
    opt = lambda k, d: args[args.index(k) + 1] if k in args else d
    langs = opt("--langs", "zh,ko,en").split(",")
    shuffles = int(opt("--shuffles", "3"))
    bodies = opt("--bodies", "male,female").split(",")
    if "--text" in args:
        lang, path = opt("--text", "").split(":", 1)
        TEXTS[lang] = [ln.strip() for ln in open(path, encoding="utf-8") if ln.strip()]
        langs = [lang]
    os.makedirs(out, exist_ok=True)
    import sherpa_onnx
    sv = {}
    results = []
    for lang in langs:
        rec = sherpa_onnx.OfflineRecognizer.from_sense_voice(model=f"{MODEL}/model.int8.onnx", tokens=f"{MODEL}/tokens.txt", language=lang, use_itn=False, num_threads=8)
        units_of = scorer(lang)
        variants = [("real", TEXTS[lang])] + [(f"scramble{s}", scramble(lang, TEXTS[lang], 100 + s)) for s in range(shuffles)]
        for body in bodies:
            for name, lines in variants:
                stem = os.path.join(out, f"{lang}_{body}_{name}")
                spans = render(lines, body, stem)
                audio = load(stem + ".wav")
                errs = total = 0
                hyps = []
                for (a, b), ln in zip(spans, lines):
                    s = rec.create_stream()
                    s.accept_waveform(16000, audio[int(a * 16000):int(b * 16000)])
                    rec.decode_stream(s)
                    r, h = units_of(ln), units_of(s.result.text)[: 2 * len(units_of(ln))]
                    errs += edits(r, h)
                    total += len(r)
                    hyps.append(s.result.text)
                results.append({"lang": lang, "body": body, "variant": name, "error": errs / total, "units": total, "hyp": hyps})
                print(f"{lang} {body:6s} {name:9s} error {errs / total:5.1%}  ({total} units)  {' | '.join(hyps)[:90]}", flush=True)
    with open(os.path.join(out, "scramble.json"), "w") as f:
        json.dump(results, f, ensure_ascii=False, indent=1)
    print("\nlang  voice   real   scrambled (mean)")
    for lang in langs:
        for body in bodies:
            rs = [x for x in results if x["lang"] == lang and x["body"] == body]
            real = next(x["error"] for x in rs if x["variant"] == "real")
            scr = [x["error"] for x in rs if x["variant"] != "real"]
            print(f"{lang:4s}  {body:6s}  {real:5.1%}  {sum(scr) / len(scr):5.1%}")


if __name__ == "__main__":
    main()
