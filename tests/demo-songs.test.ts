/**
 * The demo songs through the wasm facade: the catalogue lists them, and each one's lyrics (read
 * with the page's dictionary and Mandarin lexicon) give exactly one sung syllable per note, as
 * `playDemo` assumes (the sound is tested in crates/chimera/tests/demo_songs.rs).
 */
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { initEngine } from "./harness.ts";

await initEngine();
const { DEMO_SONGS, EnglishG2P, MandarinLexicon, lyricsToSyllables } = await import("../src/wasm/engine.ts");
new EnglishG2P().setDictionaryText(gunzipSync(readFileSync(new URL("../public/data/cmudict.txt.gz", import.meta.url))).toString());
await new MandarinLexicon().load(new URL("../public/data/mandarin.txt.gz", import.meta.url).href);

test("the catalogue lists the English, Japanese, Mandarin and Korean songs", () => {
  expect(DEMO_SONGS.map((s) => s.name)).toEqual(["Made of air", "Twinkle, twinkle", "さくら さくら", "两只老虎", "아리랑"]);
});

test("each song's lyrics give one sung syllable per note", () => {
  for (const s of DEMO_SONGS) expect({ song: s.name, syllables: lyricsToSyllables(s.lyrics).length }).toEqual({ song: s.name, syllables: s.notes.length });
});

test("a melisma mark is a note that holds the vowel before it", () => {
  for (const name of ["さくら さくら", "아리랑"]) {
    const s = DEMO_SONGS.find((d) => d.name === name)!;
    const syl = lyricsToSyllables(s.lyrics);
    expect(syl.filter((x) => x.melisma).length).toBeGreaterThan(5);
    for (const [i, x] of syl.entries()) if (x.melisma) expect(x.nucleus.ipa).toBe(syl[i - 1]!.nucleus.ipa);
  }
});
