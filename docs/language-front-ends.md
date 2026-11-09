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
| Han | Mandarin | `public/data/mandarin.txt.gz` (206 KB, loaded when lyrics have Han characters) | `phonetics/mandarin.rs` |
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

Words without tones are English. A declined word isn't re-entered midway, so `café` stays English.
The one ambiguity: an accented English word that is also valid toned pinyin (`résumé`, *ré su mé*)
reads as pinyin; type it unaccented.

`chimera syllables "<lyrics>"` prints what the front ends wrote: the quickest way to check one.

## The one format

A sung syllable per sounding note:

```json
{ "text": "话", "onset": [{"ipa": "x"}, {"ipa": "w"}], "nucleus": {"ipa": "a"}, "coda": [],
  "pitch": [{"at": "start", "len": 0.21, "max": 0.09, "st": 1}, {"at": "end", "len": 0.35, "max": 0.15, "st": -3}],
  "wordEnd": true, "phraseEnd": false }
```

- `onset` / `coda`: consonants and glides from the inventory. A stop or affricate may say
  `"aspirated": true|false`; unsaid, the planner uses English's rule. In IPA text, `ʰ` and `˭` say it.
- `nucleus`: one vowel, or a diphthong the planner knows (aɪ eɪ aʊ oʊ ɔɪ).
- `pitch`: bends that colour the note's start or end, `st` semitones from the note, over `len` of the
  vowel and at most `max` seconds. The melody stays as written.
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

The Mandarin front end's choices, A/B on the same test (8 scrambles × 2 voices, 640 syllables):

| Mandarin front end | Scrambled (male / female) | Real lyrics |
|---|---|---|
| As built: unaspirated stops said, tones as bends | **48.6%** (49.7 / 47.5) | **10%** |
| Without the tone bends | 52.8% (53.8 / 51.9) | 15% |
| Voiced stand-ins for unaspirated stops (as before) | 54.8% (55.9 / 53.8) | 20% |

Both choices help in both voices, and on real lyrics too. Saying the aspiration is worth about 6
points (two standard errors at 640 syllables). The tone bends are worth about 4.

Use Whisper large-v3 only on whole clips, with its transcript capped near the reference length: on
short clips it hallucinates subtitle credits, and on whole clips it can loop. Paraformer-zh
recognised nothing in any sung render.
