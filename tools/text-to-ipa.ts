/**
 * Text in any language → IPA lyrics the engine sings, through YOUR OWN eSpeak NG.
 *
 *   bun tools/text-to-ipa.ts --voice de "Wie schön leuchtet der Morgenstern"
 *       a line of slash-delimited IPA words, one per word eSpeak spoke, cleaned
 *   bun tools/text-to-ipa.ts --voice de --json "…"            the "syllables" array for a melody file
 *   bun tools/text-to-ipa.ts --ipa "ˈʃøːn bɔ̃ʒuʁ"            IPA from any other G2P: cleaned the same way
 *   echo "ˈʃøːn" | bun tools/text-to-ipa.ts --ipa             (or from stdin)
 *
 * Paste the lyric line into Sing, or into a melody file's `"lyrics"` (`chimera render notes`);
 * `--json` prints what `"syllables"` takes instead. Both need the wasm build (`bun run wasm`),
 * which is where the engine's own list of sounds comes from.
 *
 * eSpeak NG is NOT bundled, a dependency or imported: this runs the `espeak-ng` you installed
 * (`--espeak <path>` if it is not on PATH). It is GPL-3.0, which is why it stays outside the
 * engine. Without it, `--ipa` still works.
 *
 * Cleaning (tools/lib/text-to-ipa.ts): the language-switch annotations eSpeak writes, joiners and
 * tie bars go; stress and length stay; a symbol the engine cannot sing becomes the nearest one it
 * can through one table, a mark it does not model is dropped from its letter, and every such change
 * is printed on stderr, so you can see what the engine will sing. A word with no vowel is reported
 * too (the engine does not sing it).
 *
 * UNTESTED against a real eSpeak NG: the cleaner is written from eSpeak NG's documented `--ipa`
 * output, and its tests use hand-written examples of that format. Check the first lines you make.
 */
import { readFileSync, existsSync } from "node:fs";
import { cleanWords, formatReport, splitEspeak, toLyric } from "./lib/text-to-ipa.ts";

const USAGE = `usage: bun tools/text-to-ipa.ts [--voice <espeak voice>] [--json] [--espeak <path>] "text"
       bun tools/text-to-ipa.ts --ipa [--json] ["ipa words"]   (reads stdin without an argument)
  --voice  an eSpeak NG voice or language, e.g. de, fr, es, it, en-us (default en); espeak-ng --voices lists them
  --json   print the "syllables" array instead of the lyric line
  --ipa    the input is IPA already, not text: no eSpeak needed`;

function fail(msg: string, code = 1): never {
  console.error(msg);
  process.exit(code);
}

// ------------------------------------------------------------------ arguments
const args = process.argv.slice(2);
let voice = "en";
let espeak = "espeak-ng";
let json = false;
let ipaInput = false;
const rest: string[] = [];
for (let i = 0; i < args.length; i++) {
  const a = args[i];
  if (a === "--voice" || a === "-v") voice = args[++i] ?? fail(USAGE, 2);
  else if (a === "--espeak") espeak = args[++i] ?? fail(USAGE, 2);
  else if (a === "--json") json = true;
  else if (a === "--ipa") ipaInput = true;
  else if (a === "--help" || a === "-h") fail(USAGE, 0);
  else if (a.startsWith("--")) fail(`unknown option ${a}\n${USAGE}`, 2);
  else rest.push(a);
}
let input = rest.join(" ");
if (!input.trim()) {
  if (!ipaInput || process.stdin.isTTY) fail(USAGE, 2);
  input = await Bun.stdin.text();
}

// ------------------------------------------------------------------ the words
let raw = input;
if (!ipaInput) {
  const p = (() => {
    try {
      return Bun.spawnSync([espeak, "-q", "--ipa", "-b", "1", "-v", voice, input], { stdout: "pipe", stderr: "pipe" });
    } catch {
      return fail(`text-to-ipa: ${espeak} not found. Install eSpeak NG (https://github.com/espeak-ng/espeak-ng, or your package manager) or pass --espeak <path>; --ipa works without it.`);
    }
  })();
  if (p.exitCode !== 0) fail(`text-to-ipa: ${espeak} failed (${p.exitCode}): ${p.stderr.toString().trim()}`);
  raw = p.stdout.toString();
}
const { words: spoken, switches } = ipaInput ? { words: raw.split(/\s+/).filter(Boolean), switches: [] as string[] } : splitEspeak(raw);
if (switches.length) console.error(`text-to-ipa: eSpeak switched language mid-text (${switches.join(" ")}); the annotations are removed and the foreign words stay as eSpeak said them`);

// ------------------------------------------------------------------ the engine
const wasm = new URL("../src/wasm/chimera.wasm", import.meta.url);
if (!existsSync(wasm)) fail("text-to-ipa: the engine is not built (src/wasm/chimera.wasm): run `bun run wasm`");
const { initChimera } = await import("../src/wasm/chimera.ts");
await initChimera(readFileSync(wasm));
// engine.ts calls the wasm when it loads, so it is imported only after initChimera
const { ipaSymbols, lyricsToSyllables } = await import("../src/wasm/engine.ts");

const { words, report } = cleanWords(spoken, new Set(ipaSymbols()));
for (const line of formatReport(report)) console.error(`text-to-ipa: ${line}`);
const mute = words.filter((w) => lyricsToSyllables(toLyric([w])).length === 0);
if (mute.length) console.error(`text-to-ipa: no vowel the engine sings in: ${mute.join(" ")} (not sung)`);
if (!words.length) fail("text-to-ipa: no IPA words");

const lyric = toLyric(words);
console.log(json ? JSON.stringify(lyricsToSyllables(lyric)) : lyric);
