# Comparing the synth to a Mandarin singer: Opencpop

`tools/opencpop/compare.py` sings Opencpop's songs with a CHIMERA build and measures each rhyme
against the recording: how long each sound lasts and how a rhyme moves into its ending. It trains
nothing. It is the Mandarin counterpart of [csd-comparison.md](csd-comparison.md), built for the
question a native listener raised and the CSD comparison can't see: is a rhyme one movement or two
sounds, and how long does the singer really hold each part.

## The dataset

Opencpop (Wang et al., Interspeech 2022; <https://wenet-e2e.github.io/opencpop/>):

- one professional female singer, 100 Mandarin pop songs, about 5.2 hours in 3,756 utterances
- 44.1 kHz studio recordings
- for every utterance: the phonemes (pinyin initials and finals), each one's length, the note and
  its length, and which entries are slurs (a final carried onto another note)

It is licensed CC BY-NC-ND 4.0 and is obtained by applying through the form on its download page.
Use it for measuring only. Don't put its audio, its annotation, or anything that could rebuild
them in git; reports are written under `out/`, which is ignored.

## Running it

```sh
python tools/opencpop/compare.py ~/datasets/opencpop target/release/chimera.exe --out out/opencpop
#   --limit n | --ids a,b,...   --body female   --against out/opencpop-before/report.json
python tools/opencpop/selftest.py <singer exe> <exe under test>     # no dataset needed
```

It needs numpy and a build whose `render notes` has `--plan` (it says where the planner put each
sound). It writes `report.md`, `report.json` (the medians), `rhymes.json` (every measured rhyme)
and the renders.

## Method

**What the synth sings.** Her phonemes, at her times and her score's pitches. Each initial and
final becomes a pinyin syllable for the Pinyin front end (so the front end is under test, as it is
not in the CSD comparison). Each vowel's beat is the annotated start of its final. A note lasts to
the next vowel when the next syllable follows at once (its consonant takes its time from this note,
as in the planner), else to its own end and a rest. A slur is a melisma. One second a beat.

**Where each sound is.** Hers: the annotation. The synth's: its own plan (`--plan`), because a
synth's consonant may be shorter or longer than hers and its rhyme then ends somewhere else. A
syllable whose planned vowel is more than 30 ms from its beat is left out and counted.

**How a rhyme moves** (finals ending in an off-glide, -n or -ng, at least 150 ms long, not slurred).
Each 5 ms frame's mel cepstrum is placed on the line between the final's start (the mean of its
first fifth) and its end (the mean of its last 15%): 0 is the sound it starts with, 1 the sound it
ends with. From that position over time:

| measure | what it tells you |
|---|---|
| held (ms) | how long the ending's sound is held: from first reaching 0.9 to the final's end. A second sound held after the vowel is long here |
| t50 (ms) | when the rhyme is half-way, before the final's end |
| cross (ms) | how long the middle half of the movement takes: a jump is short |
| drop (dB) | the last 15% under the loudest part of the final: the nasal murmur, or a fading end |

Cepstra rather than formants: at a soprano's pitch the harmonics are too far apart for LPC, as in
the CSD comparison.

**Timing from the annotation alone**: how long each initial lasts (hers, and the synth's planned
onset), how long she holds each final, and her pitch in the first 200 ms of a note against the
score (how she comes into a note).

## The self-test

`selftest.py` runs the whole comparison without the dataset: made-up lines in Opencpop's format,
"recorded" by a stand-in singer that is another CHIMERA build. The same build twice gives the same
numbers in both columns. With PR #85's build as the singer and the closing-rhyme build under
test, the off-glides' *held* is 103 against 52 ms and *t50* 258 against 133 ms, and ang's *held*
178 against 87 ms: the comparison sees the difference a native listener heard.

## Caveats

- **One singer, one style.** Pop singing by one woman. Directions to tune in, not targets for
  every voice, and nothing here says how Mandarin is spoken.
- **The annotation has no boundary inside a final**, so where the off-glide or the nasal begins is
  inferred from the sound, not read from a label.
- **A real recording is not a dry synth.** Reverb tails and breaths blur *held* and *drop*; read
  them as differences between two builds against the same singer rather than as absolute targets.
- **Not run on the dataset yet.** It has been run on the self-test only.
