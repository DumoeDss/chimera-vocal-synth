# Before / after: Mandarin on top of PR #85

Evidence for the pull request "Mandarin, by a native speaker's ear". Nothing here is part of the
PR's diff. Every sound is rendered by `chimera render notes`; the song is the folk tune 两只老虎.

- `before/`: PR #85's build (58099b8), as it sings by default (its tones are drawn in every song).
- `after/`: this branch. The song is the default (the melody as written, no tones); the single
  syllables and the two sentences ask for tones (`"bendStrength": 1`).
- `tones.png`: the pitch through 妈 麻 马 骂 (ma1 to ma4), female, 0.4 s.
- `rhymes.png`: 奥 ao (F2) and 昂 ang (level) on a 1.33 s note, male.
- `numbers.txt`: the measurements quoted in the PR.

File names: `{voice}-{pinyin with tone}-{note length}.wav`, `sentence-nihao` is 你好，我是中国人
and `sentence-tianqi` is 今天天气很好 at 0.24 s a syllable on one note, `song` is 两只老虎.
