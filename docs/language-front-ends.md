# Language front ends

CHIMERA sings in two halves, the usual split in speech synthesis:

```
lyrics ──► router ──► FRONT END ──► syllables ──► BACK END ──► sound
           (by script)  English: CMUdict         (IPA + aspiration     the planner, the airway,
                        Korean: Hangul rules      + pitch bends)        the voice: language-agnostic
                        Mandarin: pinyin lexicon
                        Pinyin: typed tones
                        explicit syllables pass through
```

**A front end is phonology; the back end is physics.** A front end decides what a word's sounds
are: which phonemes, whether a stop is aspirated, how a tone bends the pitch. The back end
realizes whatever the syllables say and knows no language's rules. English's aspiration rule is the
one exception: it is still the planner's default when a front end says nothing.

## The built-in front ends

| Script | Front end | Needs | Where |
|---|---|---|---|
| Latin | English | CMUdict (letter-to-sound until it loads) | `phonetics/english.rs`, `syllables.rs` |
| Hangul | Korean | nothing: the block's letters are its sounds | `phonetics/korean.rs` |
| Han | Mandarin | `public/data/mandarin.txt.gz` (208 KB, loaded when lyrics have Han characters) | `phonetics/mandarin.rs` |
| Latin with tones | Mandarin as pinyin | nothing (checked against Mandarin's 411 syllables) | `phonetics/mandarin.rs` (`Pinyin`) |

The router (`read_lyrics` in `syllables.rs`) sends each word to the front end whose script it is
in, so a line can mix them: `我爱你，银行 hello 빵야 wǒ ài nǐ`. It also handles what every language shares:
melisma marks (`-`, `~`), phrase punctuation (ASCII and full-width `，。！？`) and word ends. Type
lyrics in Sing mode, a melody file's `"lyrics"`, or a duet show's melody files.

Pinyin and English share the Latin alphabet, so a front end may decline a word it scanned. Pinyin
claims only a word that carries tones and parses fully into real syllables:
- **Tone marks:** `nǐhǎo`, `xièxie` (an unmarked syllable inside a marked word is neutral).
- **Tone numbers:** `ni3 hao3`, `de5` or `de0`.
- **Splitting:** apostrophes split syllables (`Xī'ān`), and `ü` or `v` both work.
- **Tone changes:** third-tone sandhi applies inside a word (`nǐhǎo` sings nǐ rising). Write 不 and 一
  with the tone they're sung with (`bú`, `yí`), since pinyin alone can't tell 不 from 部.

Both Mandarin front ends sing erhua as one syllable ending in ɻ: 一点儿 is 一 + 点儿 (pinyin
`yìdiǎnr`), 哪儿 is `nǎr`. A 儿 that is a word's own syllable (女儿, 儿子) stays a syllable.

Words without tones are English. A declined word isn't re-entered midway, so `café` stays English.
The one ambiguity: an accented English word that is also valid toned pinyin (`résumé`, *ré su mé*)
reads as pinyin; type it unaccented.

`chimera syllables "<lyrics>"` prints what the front ends wrote: the quickest way to check one.

## The one format

A sung syllable per sounding note:

```json
{ "text": "话", "onset": [{"ipa": "x"}], "nucleus": {"ipa": "ua"}, "coda": [],
  "pitch": [{"at": "start", "len": 0.21, "max": 0.09, "st": 1}, {"at": "end", "len": 0.35, "max": 0.15, "st": -3}],
  "wordEnd": true, "phraseEnd": false }
```

- `onset` / `coda`: consonants and glides from the inventory. A stop or affricate may say
  `"aspirated": true|false` (unsaid: English's rule; `ʰ` and `˭` in IPA text) and `"vot"`, how long
  its aspiration lasts in seconds (unsaid: 50 ms). Any consonant may say its sung `"dur"` (s;
  unsaid: its spoken length × 1.25): that's how a language holds its nasal codas or keeps an
  affricate's frication short.
- `nucleus`: one vowel, or a diphthong the planner knows. A note sings it as an on-glide, a held
  vowel and an off-glide. Falling ones (aɪ eɪ aʊ oʊ ɔɪ əʊ) hold their first vowel and glide off in
  the last 35% of the note. Rising ones (ia iɛ iɑ iʊ ua uo uɔ uɑ uə yɛ) sing a short glide on the
  beat, a quarter of the vowel and at most 120 ms, then hold the second vowel. Triphthongs (iaʊ
  iəʊ uaɪ ueɪ) do both. Use them for medial glides rather than putting the glide in the onset,
  where it's squeezed before the beat with the consonants. The apical vowels ɿ (四) and ʅ (是) are
  calibrated vowels too.
- `pitch`: bends that colour the note's start or end, `st` semitones from the note, over `len` of the
  vowel and at most `max` seconds. The melody stays as written. A start bend begins in the onset,
  so a rising tone starts low rather than starting at the note and dipping.
- `breath`: lung drive while the note sounds, relative to the phrase's (unsaid: the phrase's).
- `nasalLead`: nasalise this fraction of the vowel before a nasal coda (unsaid: the song's
  `nasalLead`, off by default).
- A melisma note is `"melisma": true` with the same nucleus; its coda moves to the run's last note.
- `text` is what the mouth, the lyrics panel and the piano roll show.

Every front end's output can be frozen to JSON and edited, then sung with `"syllables"` instead of
`"lyrics"`. This works in `render notes`, `DemoSong.syllables` and duet melody files. Use it to
fix a reading by hand (了 as *liǎo* instead of *le*), or to sing a language before it has a front
end.

## Adding a language

1. **Map its sounds onto the inventory** (`phonetics/inventory.rs`; calibrated vowels in
   `vowelTargets.generated.json`). Add missing ones between their neighbours, as ɕ ʑ tɕ dʑ tʂ dʐ ɥ
   were, and let `chimera calibrate consonants` set a fricative's constriction: it appends new
   entries and leaves the existing ones alone.
2. **Write the front end**: implement `FrontEnd` (which characters start and continue a word, and
   word → syllables), then add it to the router lists (`lyrics_to_syllables` for front ends without
   data; the wasm engine's and the CLI's `read_lyrics` calls for ones with data). Front ends with
   data load it like CMUdict: a gzipped file in `public/data/`, fetched only when lyrics need it.
3. **Say the phonology**: aspiration where the language contrasts it, and pitch bends for tones or
   accent. Construct `Syllable`s with `..Default::default()` so later fields don't break you.
4. **Run the scramble test** before believing it works.

Han characters are shared by Mandarin, Cantonese and Japanese. With a second Han front end, a
song-level language choice would pick between them; today Mandarin claims them.

## The scramble test

A recogniser that knows a lyric fills it in. So the test that counts sings the same melody with the
lyric's units shuffled (syllables for Chinese and Korean, words for English): nothing to guess from.

`tools/asr/scramble.py` (set up with `tools/asr/setup.sh`) does this for every built-in front end:
- **Material:** public-domain texts. Two Tang poems (静夜思, 春晓), the folk song 아리랑, and Harvard
  sentences.
- **Rendering:** each goes through CHIMERA's own front end, on one generated tune with one note per
  syllable and a breath between lines, in the male and female voices.
- **Judge:** SenseVoice-Small (sherpa-onnx), one sung line at a time. It's CTC, so it can't
  hallucinate a loop, and it covers Chinese, English, Japanese, Korean and Cantonese.
- **Score:** syllable errors (Chinese as toneless pinyin, Korean as Hangul blocks) or word errors
  (English).

Results, 3 scrambles × 2 voices per language:

| Language | Real lyrics (male / female) | Scrambled (male / female) |
|---|---|---|
| Chinese (syllables) | 7.5% / 12.5% | 50% / 44% |
| Korean (syllables) | 50% / 28% | 59% / 52% |
| English (words) | 49% / 61% | 61% / 76% |

- **Familiar text gets help.** The poems are known by heart, so the real lyrics come back almost
  perfectly; scrambled, the same sounds lose about 40 points.
- **Every language is limited by the sung voice.** On sound alone they all land around 45–75%
  (English's words are a stricter unit than syllables).

Pass `--text lang:file` to run your own lines: material dense in what a change touches is the only
way to see it. The poems above have almost no apical, vowel-initial or neutral-tone syllables, so
changes to those are invisible on them.

The Mandarin front end's choices, A/B on the same test (8 scrambles × 2 voices, 640 syllables):

| Mandarin front end | Scrambled (male / female) | Real lyrics |
|---|---|---|
| As built: unaspirated stops said, tones as bends | **48.6%** (49.7 / 47.5) | **10%** |
| Without the tone bends | 52.8% (53.8 / 51.9) | 15% |
| Voiced stand-ins for unaspirated stops (as before) | 54.8% (55.9 / 53.8) | 20% |

Both choices help in both voices, and on real lyrics too. Saying the aspiration is worth about 6
points (two standard errors at 640 syllables). The tone bends are worth about 4.

Then the front end's mouth and breath, one change at a time, on the poems or on lines written to be
dense in what each touches:

| Change | Text | Scrambled before → after | Kept |
|---|---|---|---|
| Medials inside the vowel (rising diphthongs) | poems | 48.6% → **44.2%** | yes |
| Apical vowels ɿ ʅ instead of [ɨ] | apical-dense lines | 66.3% → **46.2%** | yes |
| Glottal catch before a vowel-initial syllable | vowel-initial lines | 61.9% → **57.8%** | yes |
| 90 ms aspiration (`vot`) | poems | 48.6% → 49.2% | yes (neutral; Mandarin's is ~80–100 ms) |
| Nasalised vowels before -n/-ng | poems | neutral | yes |
| Neutral tones at 0.7 breath | vowel-initial lines | 61.2% → 61.9%, real lines worse | no |
| Lips following the vowel for x/j/q | poems | 48.6% → 51.1% | no (spread lips sharpen ɕ) |

The biggest wins came from the sounds the recogniser confused most: the apical vowels (是 heard as
虚) and the medial glides (话 drifting toward 哈).

### Reading what goes wrong

The test also aligns each sung line's syllables with what was heard and reports the confusions:
how often each part of a syllable is wrong (Chinese: initial, final, tone; Korean: initial, vowel,
final) and the top confusion pairs. `--analyze <scramble.json>` re-reads a run. tools/asr/probes/
has lines dense in one contrast each (sibilants, nasal codas, vowel-initial syllables, apical
vowels) for `--text`.

That report drove the next round, each change aimed at a named confusion (8 scrambles × 2 voices,
cumulative, on the poems):

| Change | Confusion it targeted | Scrambled |
|---|---|---|
| start of the round | | 43.9% |
| nasal codas held (-ng 130 ms, -n 100 ms) | | 41.6% |
| ʅ recalibrated (F2 1500, F3 2000) | zhi/shi heard as qu ×22 → ×8 | 40.9% |
| iao iou uai uei as one nucleus | niao heard as yao ×16 → ×7 | 42.5% |
| uo [uɔ], ou [əʊ] | uo/ou heard as u ×26 → 0 | 41.7% |
| onset timing (j/zh/z short, n/m and f long) | j heard as q ×19 → ×12 | 40.0% |
| eng [ɤŋ] | eng heard as en ×12 → ×7 | 37.8% |
| stronger tones, start bends in the onset | rising heard as level ×44 → ×29 | **35.8%** |

What's left is mostly engine-bound: -ng heard as -n (uang→uan ×16), and the sibilants (sh→f/s/ch,
s→sh). Those live in the noise source and the nasal murmur, not the front end.

English's report (phonemes via CMUdict: the reference against the heard words) showed something else:
sounds *dropped* rather than confused, 29% of vowels and 28% of consonants, and word-initial
consonants most (58% of them lost). Sung consonants were 0.8× their spoken length, too short for a
held note. Same loop, Harvard sentences (`probes/en_harvard.txt`), 8 scrambles × 2 voices:

| Change | Confusion it targeted | English | Mandarin poems | Korean 아리랑 |
|---|---|---|---|---|
| before | | 71.5% | 34.5% | 59.2% |
| sung consonants 1.0× | dropped onsets | 69.2% | | |
| sung consonants 1.25× | dropped onsets; D→T ×20 → ×10 | 68.1% | 33.9% | **56.6%** |
| sung consonants 1.5× | | 70.7% (overshoots) | | |
| 1.25×, and off-glides over 35% of the note (was 20%) | EY heard as IH, OW as AH | **66.1%** | **30.6%** | 56.6% (no diphthongs) |

Both are now the planner's defaults; a live keyboard note keeps short consonants for latency. What
English still gets wrong: AH heard as ER (her, were), AA as AH/AE, DH as L, and SH/S as F: the same
weak sibilant noise as Mandarin's sh→f.

Score-level tries at -n/-ng, none kept (nasal probe, 8 scrambles × 2 voices; baseline 59.2 / 52.1%,
finals wrong 31%, in→ing ×28, ing→in ×18):
- **A phrase-final nasal fading out closed** (no oral release while voiced): 64.6 / 52.1%, in→ing
  ×35. The release, though unnatural, carries place.
- **ing as [iəŋ]:** ing→in ×18 → ×6, but heard as ian ×18 instead; 58.8 / 51.7%.
- **The tongue backing toward the velum in ing's vowel** (TbX 3.0): 60.4 / 51.7%.
- **No tongue-body fronting during a coda -n:** 60.4 / 52.9%.

All within the test's noise. On this probe the real lyrics also come back 57–60% wrong: in/ing may
be near what this recogniser separates in singing.

Use Whisper large-v3 only on whole clips, with its transcript capped near the reference length: on
short clips it hallucinates subtitle credits, and on whole clips it can loop. Paraformer-zh
recognised nothing in any sung render.
