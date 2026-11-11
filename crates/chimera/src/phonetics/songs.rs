//! Demo melodies for Sing mode: [MIDI note, beats] per syllable, in the adult female register.

use serde::Serialize;

/// octave shift per body
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongOctave {
    pub male: f64,
    pub female: f64,
    pub child: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoSong {
    pub name: &'static str,
    pub bpm: f64,
    pub lyrics: &'static str,
    pub notes: &'static [[f64; 2]],
    /// octave shift per body
    pub octave: SongOctave,
}

#[rustfmt::skip]
pub static DEMO_SONGS: &[DemoSong] = &[
    DemoSong {
        // an original tune for this instrument
        name: "Made of air",
        bpm: 84.0,
        lyrics: "Breathe in, breathe out, I am made of air. Tongue and lips and throat, singing anywhere.",
        notes: &[
            [64.0, 1.0], [60.0, 1.0], [62.0, 1.0], [57.0, 1.0],
            [57.0, 0.5], [60.0, 0.5], [64.0, 1.0], [62.0, 0.5], [60.0, 1.5],
            [65.0, 1.0], [64.0, 0.5], [62.0, 0.5], [60.0, 0.5], [62.0, 1.5],
            [64.0, 0.5], [62.0, 0.5], [60.0, 0.5], [59.0, 0.5], [57.0, 2.0],
        ],
        octave: SongOctave { male: -1.0, female: 0.0, child: 0.0 },
    },
    DemoSong {
        // traditional (public domain)
        name: "Twinkle, twinkle",
        bpm: 100.0,
        lyrics: "Twinkle twinkle little star, how I wonder what you are.",
        notes: &[
            [60.0, 1.0], [60.0, 1.0], [67.0, 1.0], [67.0, 1.0], [69.0, 1.0], [69.0, 1.0], [67.0, 2.0],
            [65.0, 1.0], [65.0, 1.0], [64.0, 1.0], [64.0, 1.0], [62.0, 1.0], [62.0, 1.0], [60.0, 2.0],
        ],
        octave: SongOctave { male: -1.0, female: 0.0, child: 0.0 },
    },
    DemoSong {
        // traditional (public domain): the koto piece "Saita sakura", words of the 1888 school songbook
        // (anonymous). One note per mora; ー holds the vowel of the kana before it.
        name: "さくら さくら",
        bpm: 84.0,
        lyrics: "さくら さくら、やよいの そらーわ、みわたす かぎーり、かすみか くもーか、においぞ いずーる、いざや いざや、みにゆー かーん。",
        notes: &[
            [69.0, 1.0], [69.0, 1.0], [71.0, 2.0],
            [69.0, 1.0], [69.0, 1.0], [71.0, 2.0],
            [69.0, 1.0], [71.0, 1.0], [72.0, 1.0], [71.0, 1.0],
            [69.0, 1.0], [71.0, 0.5], [69.0, 0.5], [65.0, 2.0],
            [64.0, 1.0], [60.0, 1.0], [64.0, 1.0], [65.0, 1.0],
            [64.0, 1.0], [64.0, 0.5], [60.0, 0.5], [59.0, 2.0],
            [69.0, 1.0], [71.0, 1.0], [72.0, 1.0], [71.0, 1.0],
            [69.0, 1.0], [71.0, 0.5], [69.0, 0.5], [65.0, 2.0],
            [64.0, 1.0], [60.0, 1.0], [64.0, 1.0], [65.0, 1.0],
            [64.0, 1.0], [64.0, 0.5], [60.0, 0.5], [59.0, 2.0],
            [69.0, 1.0], [69.0, 1.0], [71.0, 2.0],
            [69.0, 1.0], [69.0, 1.0], [71.0, 2.0],
            [64.0, 1.0], [65.0, 1.0], [71.0, 0.5], [69.0, 0.5], [65.0, 1.0], [64.0, 2.0],
        ],
        octave: SongOctave { male: -1.0, female: 0.0, child: 0.0 },
    },
    DemoSong {
        // traditional (public domain): the French canon "Frère Jacques" with the anonymous Chinese
        // children's rhyme
        name: "两只老虎",
        bpm: 100.0,
        lyrics: "两只老虎，两只老虎，跑得快，跑得快，一只没有耳朵，一只没有尾巴，真奇怪，真奇怪。",
        notes: &[
            [65.0, 1.0], [67.0, 1.0], [69.0, 1.0], [65.0, 1.0],
            [65.0, 1.0], [67.0, 1.0], [69.0, 1.0], [65.0, 1.0],
            [69.0, 1.0], [70.0, 1.0], [72.0, 2.0],
            [69.0, 1.0], [70.0, 1.0], [72.0, 2.0],
            [72.0, 0.5], [74.0, 0.5], [72.0, 0.5], [70.0, 0.5], [69.0, 1.0], [65.0, 1.0],
            [72.0, 0.5], [74.0, 0.5], [72.0, 0.5], [70.0, 0.5], [69.0, 1.0], [65.0, 1.0],
            [65.0, 1.0], [60.0, 1.0], [65.0, 2.0],
            [65.0, 1.0], [60.0, 1.0], [65.0, 2.0],
        ],
        octave: SongOctave { male: -1.0, female: 0.0, child: 0.0 },
    },
    DemoSong {
        // traditional (public domain): the refrain and first line of "Bonjo Arirang", in 9/8 (an eighth
        // is half a beat). ~ holds the syllable over the next note.
        name: "아리랑",
        bpm: 84.0,
        lyrics: "아리랑 ~ 아리랑 ~ 아라 ~ ~ 리 ~ 요 ~ ~ ~ , 아리랑 ~ 고 ~ 개 ~ 로 ~ 넘어간 다 .",
        notes: &[
            [60.0, 2.5], [62.0, 0.5], [60.0, 1.0], [62.0, 0.5],
            [65.0, 2.5], [67.0, 0.5], [65.0, 1.0], [67.0, 0.5],
            [69.0, 1.5], [67.0, 0.5], [69.0, 0.5], [67.0, 0.5], [65.0, 1.0], [62.0, 0.5],
            [60.0, 2.5], [62.0, 0.5], [60.0, 0.5], [62.0, 0.5],
            [65.0, 2.5], [67.0, 0.5], [65.0, 1.0], [67.0, 0.5],
            [69.0, 1.0], [67.0, 0.5], [65.0, 1.0], [62.0, 0.5], [60.0, 1.0], [62.0, 0.5],
            [65.0, 2.5], [67.0, 0.5], [65.0, 1.5],
            [65.0, 3.0],
        ],
        octave: SongOctave { male: -1.0, female: 0.0, child: 0.0 },
    },
];
