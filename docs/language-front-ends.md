# Language front ends

CHIMERA sings in two halves, the usual split in speech synthesis:

```
lyrics ──► router ──► FRONT END ──► syllables ──► BACK END ──► sound
           (by script)  English: CMUdict         (IPA + aspiration     the planner, the airway,
                        Korean: Hangul rules      + pitch bends)        the voice: language-agnostic
                        Japanese: kana rules
                        Mandarin: pinyin lexicon
                        Pinyin: typed tones
                        IPA: words between slashes
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
| Kana | Japanese | nothing: the kana are their sounds | `phonetics/japanese.rs` |
| IPA in `/slashes/` | IPA | nothing: the IPA is the sounds, split by sonority | `phonetics/ipa_lyrics.rs` |
| Han | Mandarin | `public/data/mandarin.txt.gz` (208 KB, loaded when lyrics have Han characters) | `phonetics/mandarin.rs` |
| Latin with tones | Mandarin as pinyin | nothing (checked against Mandarin's 411 syllables) | `phonetics/mandarin.rs` (`Pinyin`) |

The router (`read_lyrics` in `syllables.rs`) sends each word to the front end whose script it is
in, so a line can mix them: `我爱你，银行 hello 빵야 wǒ ài nǐ`. It also handles what every language shares:
melisma marks (`-`, `~`), phrase punctuation (ASCII and full-width `，。！？`) and word ends. Type
lyrics in Sing mode, a melody file's `"lyrics"`, or a duet show's melody files.

Japanese is typed in kana (hiragana, katakana, half-width katakana), one note per mora: `さくら`,
`コーヒー`, `きょう`. It is its own section below; kanji are not read. Any other language is typed
as IPA between slashes: `/ˈʃtʁaːsə/` (see "IPA lyrics").

Pinyin and English share the Latin alphabet, so a front end may decline a word it scanned. Pinyin
claims only a word that carries tones and parses fully into real syllables:
- **Tone marks:** `nǐhǎo`, `xièxie` (an unmarked syllable inside a marked word is neutral).
- **Tone numbers:** `ni3 hao3`, `de5` or `de0`.
- **Splitting:** apostrophes split syllables (`Xī'ān`), and `ü` or `v` both work.
- **Tone changes:** third-tone sandhi applies inside a word (`nǐhǎo` sings nǐ rising). Write 不 and 一
  with the tone they're sung with (`bú`, `yí`), since pinyin alone can't tell 不 from 部.

Mandarin's er (耳 ěr, 二 èr, 儿 ér) is a central vowel moving into the r-colouring, [əɚ], with 二 more
open, [aɚ]: the rising nuclei `əɝ` and `aɝ`, a quick [ə] or [a] before the beat and the r-coloured ɝ
held from it. English's /ɝ/ held still put F3 low from the first frame; here it falls from ~2300 Hz at
the beat to ~1870 over 120 ms (`tests/er.rs`).

Mandarin's u (乌 wu, 虎 hu, and the u of 话 hua, 朵 duo, 光 guang, 对 dui) is its own vowel, `u̠` (u + U+0320),
not English's /u/ "who'd": Beijing speakers' u has F2 ~660 Hz against ~1000 for the English one, which a
native listener hears as an accent (Zee & Lee 2001, see the References in `architecture.md`). Its table
entry is calibrated to their means (`vowel_data.rs`: men 345 / 661 Hz F1 / F2, women 345 / 759). The tube
reaches F2 within 3 % but not F1 (men +11 %, women +6 %: tbX, jaw and lip width are at their lower bounds, lip
protrusion at its upper), and the key can't be typed in IPA lyrics (the tokenizer drops the mark). The rising
nuclei with that glide are `u̠a u̠ɔ u̠ɑ u̠ə` and the triphthongs `u̠aɪ u̠eɪ`, beside the plain `ua uɔ uɑ uə uaɪ ueɪ`
(those keep English's u). `tests/mandarin_u.rs`.

Mandarin's ong (东 dong, 中 zhong, 兄 xiong) is `ʊ̠` (ʊ + U+0320), Zee & Lee's "u before ŋ" (men 441 / 730 Hz
F1 / F2, women 533 / 945) rather than English /ʊ/ "hood" (F2 ~1120). It is also the off-glide of ao and ou
(the nuclei `aʊ̠`, `əʊ̠`) and of iao, iou and iong (`iaʊ̠`, `iəʊ̠`, `iʊ̠`); ou's central ə start is as it was.
The fit reaches F1 and F2 within 5 % on every body (`vowel_data.rs`). `tests/mandarin_ong.rs`.

Both Mandarin front ends sing erhua as one syllable ending in ɻ: 一点儿 is 一 + 点儿 (pinyin
`yìdiǎnr`), 哪儿 is `nǎr`. A 儿 that is a word's own syllable (女儿, 儿子) stays a syllable.

Words without tones are English. A declined word isn't re-entered midway, so `café` stays English.
The one ambiguity: an accented English word that is also valid toned pinyin (`résumé`, *ré su mé*)
reads as pinyin; type it unaccented.

`chimera syllables "<lyrics>"` prints what the front ends wrote: the quickest way to check one.

## Japanese

`phonetics/japanese.rs`. Kana only: Han characters stay with the Mandarin front end (see below), so
type kanji lyrics as kana. There is no kanji dictionary. One kana is one note, and the note's text
is the kana as typed; a small ゃゅょ or small vowel joins the kana before it (`きゃ`, `ふぁ`).
- **Sounds:** し ɕ, ち tɕ, つ ts, ふ ɸ, ひ ç, に ɲ, じ/ぢ dʑ, ず/づ z, ら行 ɾ, を o. Yōon: `きゃ` is
  k j a, and しゃ ちゃ じゃ にゃ ひゃ carry their palatal (ɕa). Loanword kana: ふぁ ɸa, てぃ ti, うぃ wi,
  ちぇ tɕe, ヴ v and the like.
- **Aspiration:** a weak, short one (30 to 40 ms) on every voiceless stop and affricate, said
  explicitly: Japanese has no aspirated/unaspirated contrast, and English's positional rule would
  make the k of `きゃ` (k j a) differ from the k of `か`.
- **ん** is the previous note's coda (the text shows both: `かん`), held a mora long and placed by
  what follows: m before p b m, n before t d n ts z tɕ dʑ ɾ, ŋ before k g, ɴ at the end of a word and
  before a vowel, glide or fricative. A ん that starts a word, or follows another ん, is its own note:
  the nasal and ɯ.
- **っ** holds the next consonant twice as long, through its sung `dur` (no second stop); its kana
  joins the previous note's text. Before a vowel, ɾ or a glide, and at the end of a word, it is a
  glottal stop coda. At the start of a word it is dropped.
- **Long vowels** are melisma notes that continue the vowel, one per kana: ー, a vowel kana that
  repeats the vowel before it in the word (`おかあさん`, `おおきい`), and the spellings おう → oo, えい → ee.
  The rule can't tell a real う (the う of 思う sings as a continued o).
- **は and へ** are read as written; type `わ` and `え` for the particle readings.
- **No pitch accent and no vowel devoicing:** the notes carry the pitch, and a devoiced vowel would
  be a silent note.
- Iteration marks (ゝ ゞ) repeat the kana before them; ・ separates words.
- **Not yet checked** with the scramble test or by a native speaker.

## IPA lyrics

`phonetics/ipa_lyrics.rs`. A word between slashes is IPA, in Sing, `chimera syllables`, `render notes`
and duet melody files, mixed freely with the other front ends: `さくら /ˈʃøːn/ hello`. Everything up to
the closing slash on the same line is one word; a space inside it starts another (`/ˈhɛ loʊ/`). An
unclosed slash is dropped.
- **Syllables, with no onset table.** Every vowel or diphthong the planner knows is a nucleus.
  Consonants before the first are its onset and after the last its coda (`/ˈpsixa/` and `/ˈmnɛ/` keep
  their clusters). Between two nuclei the longest run of consonants at the end whose sonority rises
  goes to the next syllable, and the rest closes the previous one; sonority is the inventory's manner
  (stop, affricate < fricative < nasal < liquid < approximant). So `/ˈɡɾasjas/` is `ɡɾa.sjas`,
  `/ˈasta/` is `as.ta`, and `/ˈtʃaːo/` is `tʃaː.o`. This is a rule, not a language: write a `.` where
  a language says otherwise (`/ˈa.spa/`). `,` and `|` break too. Stress marks do not move a break
  (eSpeak NG puts them before the vowel, the IPA convention before the syllable).
- **Marks:** stress `ˈ ˌ`, length `ː`, tie bars, and the marks that change how a sound is made:
  `ʰ` / `˭` for aspiration, a combining tilde on a vowel (`/bɔ̃ʒuʁ/`, nasalized), `ʲ` after a
  consonant (palatalized), `ʱ` after a voiced stop or affricate and a combining diaeresis below on a
  vowel (breathy); "The one format" says what each does. IPA lyrics are narrow transcription: an
  unmarked voiceless stop or affricate is unaspirated, so `/pa/` and `/pʰa/` contrast, instead of
  English's positional rule deciding. A note's text is the syllable's IPA as the engine read it
  (marks kept, tie bars gone, `ɡ` shown as `g`).
- **Nothing is dropped silently.** The engine has no devoicing, no `ʏ` or `ɜ`, and the tokenizer drops
  what it doesn't know. `chimera syllables` and `render notes` print a warning on stderr naming the
  symbols it doesn't know, the symbols it has no sound for, the marks it ignores (a ring on `n̥`, a
  tilde on a consonant) or can't apply (`ʲ` after a vowel), and the words with no vowel (not sung).
  A nasal vowel is not reported: it is sung nasal. In code, `unknown_ipa`, `ignored_ipa_marks` and
  `ipa_lyric_report` (the crate), `sung_symbols` and `sung_marks` for what can be written. Sing mode
  does not show these yet.

### A language without a front end

`tools/text-to-ipa.ts` turns text into such a line with an eSpeak NG you install yourself:

```
bun tools/text-to-ipa.ts --voice de "Wie schön leuchtet der Morgenstern"   # /…/ /…/ …, for Sing or "lyrics"
bun tools/text-to-ipa.ts --voice de --json "…"                              # the "syllables" array instead
bun tools/text-to-ipa.ts --ipa "ˈʃøːn bɔ̃ʒuʁ"                               # IPA from any other G2P, no eSpeak
```

eSpeak NG is GPL-3.0 and large, so it is not bundled, not a dependency and not imported: the script
runs the `espeak-ng` on your PATH (or `--espeak <path>`), and says how to get one when it is missing.
It cleans what eSpeak writes: language-switch annotations, joiners and tie bars go; symbols the engine
can't sing become the nearest it can through one table (`ʏ` → `y`, `ɔʏ` → `ɔɪ`, `ɐ̯` → `ɐ`, ...), a
mark the engine doesn't read on its letter is dropped from it (a ring; a nasal vowel keeps its
tilde, `ʲ ʱ ʰ` stay), and every change is printed on stderr. No sung symbol is ever stripped (`ç`
stays `ç`). The engine's list of sounds and of marks (with the sounds each applies to) is read from
the wasm build (`bun run wasm`), so the table cannot drift: its test fails when the engine gains a
sound or mark an entry replaces.

**Not tested with a real eSpeak NG:** the cleaner follows eSpeak's documented `--ipa` output and its
tests use hand-written examples of that format. Check the first lines you make, and what the stderr
report says was changed. The nearest-sound choices are guesses, not a native speaker's.

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
  affricate's frication short. Any of them may carry the sound marks of IPA text: a consonant
  `"palatalized": true` (`ʲ`) and a voiced stop or affricate `"breathy": true` (`ʱ`).
- `nucleus`: one vowel, or a diphthong the planner knows. A note sings it as an on-glide, a held
  vowel and an off-glide. Falling ones (aɪ eɪ aʊ oʊ ɔɪ əʊ) hold their first vowel and glide off in
  the last 35% of the note. Rising ones (ia iɛ iɑ iʊ ua uo uɔ uɑ uə yɛ, and Mandarin's er: əɝ, aɝ for 二) sing the first vowel as a quick
  glide of 50 ms that ends on the beat, after the initial consonant, so the second vowel holds from
  the beat. Triphthongs (iaʊ iəʊ uaɪ ueɪ) do both. Use them for medial glides rather than putting
  the glide in the onset as a consonant (j w ɥ), where the planner treats it as a consonant. The
  glide is not held on the beat: sung as a vowel of its own there (it was up to 120 ms) the jaw and
  tongue sat at its posture for 80-100 ms and the main vowel was 90% reached 200 ms after the
  beat, so a native listener heard 两 as *li-ang* and 快 as *ku-ai*. Now the main vowel is 90%
  reached 80-90 ms after the beat (50 ms without a medial), `tests/medials.rs`. The apical vowels ɿ (四) and ʅ (是) are
  calibrated vowels too, but made with the tongue tip, not fitted to their formants alone: the
  calibrator (`chimera calibrate vowels`) pins their lips (spread and open as for [i], no narrowing or
  protrusion) and their tongue tip (the place of the sibilant before them, ɿ at s's, ʅ at ʂ's and
  curled, opened 0.11 in TtY so the constriction is ~0.3 cm², a vowel's) and fits the tongue body,
  jaw, root and larynx around that. The tip hardly moves from the consonant into the vowel (tip
  y 0.05-0.28 apart, was 1.7-2.5; `tests/apical.rs`). Fitted to formants alone it hit them with
  lips narrowed to 0.2 aperture and no tip at all, and native listeners heard 只 as zhu, zhe or zhi. A vowel may say `"nasalized": true` (a combining tilde: the velum lowers for
  the whole vowel, glides included) and `"breathy": true` (a diaeresis below: the folds part-open).
- **Sound marks** (`nasalized`, `palatalized`, `breathy`, with `aspirated`): the planner turns each
  into gestures on the velum, the tongue body or the glottis, and a mark on a sound it doesn't apply
  to does nothing. What each does is in [architecture.md](architecture.md), section 4.
- `pitch`: bends that colour the note's start or end, `st` semitones from the note, over `len` of the
  vowel and at most `max` seconds. The melody stays as written. A start bend begins in the onset,
  so a rising tone starts low rather than starting at the note and dipping. A sung song does not
  draw them by default: see "Tone bends are off in singing".
- `breath`: lung drive while the note sounds, relative to the phrase's (unsaid: the phrase's).
- `nasalLead`: nasalise this fraction of the vowel before a nasal coda (unsaid: the song's
  `nasalLead`, off by default).
- A melisma note is `"melisma": true` with the same nucleus; its coda moves to the run's last note.
- `text` is what the mouth, the lyrics panel and the piano roll show.

Every front end's output can be frozen to JSON and edited, then sung with `"syllables"` instead of
`"lyrics"`. This works in `render notes`, `DemoSong.syllables` and duet melody files. Use it to
fix a reading by hand (了 as *liǎo* instead of *le*), or to sing a language before it has a front
end (`tools/text-to-ipa.ts --json` writes one).

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
song-level language choice would pick between them; today Mandarin claims them, and Japanese is
typed in kana.

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

### Tone bends are off in singing

The front end still writes a Mandarin tone's bends into each syllable's `pitch` (2 and 3 scoop up
from 3.5 and 4 semitones below, 4 starts 1.5 above and drops 4 at the note's end), and `tone_bends`
and its tests are unchanged. But the sing planner draws them only as strongly as the song asks:
`SongOptions::bend_strength`, `"bendStrength"` in a melody file, `bendStrength` in the app's
`SongExtras`. The default is 0, the melody as written; 1 draws the bends exactly as the front end
wrote them (`tests/bends.rs`); in between scales their semitones.

Why: the ASR score preferred the bends (about 4 points, table above) and the ears did not. A native
Mandarin speaker listening to the demo song heard "a foreigner reciting the tones while singing",
and four independent listening diagnoses agreed with him: the scoops and the fall at a note's end
sound out of tune against the tune. In sung Mandarin the melody overrides the lexical tone, and
the tone is carried by the words and the context. The scramble numbers above were measured with
bends on; to reproduce them pass `--melody '{"bendStrength": 1}'` to `tools/asr/scramble.py`.

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
