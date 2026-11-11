/**
 * Japanese through the wasm facade: kana typed in Sing mode reach the engine's front end, in one
 * line with other scripts (the sounds themselves are tested in crates/chimera/tests/japanese.rs).
 */
import { expect, test } from "bun:test";
import { initEngine } from "./harness.ts";

await initEngine();
const { lyricsToSyllables } = await import("../src/wasm/engine.ts");

test("kana are sung, one note per mora, beside Latin and Hangul words", () => {
  const s = lyricsToSyllables("さくら、がっこう 빵야 hello");
  expect(s.slice(0, 6).map((x) => x.text)).toEqual(["さ", "く", "ら", "がっ", "こ", "う"]);
  expect(s[2]!.phraseEnd).toBe(true);
  expect(s[3]!.coda).toEqual([]);
  expect(s[4]!.onset[0]!.dur).toBeGreaterThan(0.15); // the k after っ is held
  expect(s[5]!.melisma).toBe(true); // こう sings oo
  expect(s.slice(6, 8).map((x) => x.text)).toEqual(["빵", "야"]);
  expect(s.length).toBeGreaterThan(8);
});

test("ん is the coda of the note before it", () => {
  const s = lyricsToSyllables("かんぱい");
  expect(s.map((x) => x.text)).toEqual(["かん", "ぱ", "い"]);
  expect(s[0]!.coda.map((c) => c.ipa)).toEqual(["m"]);
});
