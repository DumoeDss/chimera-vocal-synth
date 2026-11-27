/**
 * Voice presets: a body and the tone controls the app already has (Pressed, Breathy, Bright,
 * Rough, the vibrato's rate and Sing's vibrato depth), set together from one menu. They are
 * everyday voices to pick from, not imitations of anyone (#184).
 */
import { DEFAULT_SOURCE, type VoiceSource } from "../wasm/engine.ts";
import { PRESETS, sameBody, type BodyMix } from "./body.ts";
import { useStore } from "./store.ts";
import { DEFAULT_VIBRATO, useSing } from "./singing.ts";

/** The source values a preset sets; the rest (pitch, breath pressure, glide, fry) stay as they are. */
export type PresetTone = Pick<VoiceSource, "tension" | "breathiness" | "brightness" | "roughness" | "vibratoRate">;

export interface VoicePreset {
  id: string;
  label: string;
  body: BodyMix;
  tone: PresetTone;
  /** Sing's vibrato depth on held vowels (cents) */
  vibrato: number;
}

const TONE_KEYS = ["tension", "breathiness", "brightness", "roughness", "vibratoRate"] as const;

/** The app's own tone: what the body presets sing with. */
export const DEFAULT_TONE: PresetTone = {
  tension: DEFAULT_SOURCE.tension,
  breathiness: DEFAULT_SOURCE.breathiness,
  brightness: DEFAULT_SOURCE.brightness,
  roughness: DEFAULT_SOURCE.roughness,
  vibratoRate: DEFAULT_SOURCE.vibratoRate,
};

export const VOICE_PRESETS: readonly VoicePreset[] = [
  // hand-tuned by ear (there is no child singing data to fit): a little breath, no vibrato
  {
    id: "child1",
    label: "Child 1",
    body: PRESETS.child,
    tone: { tension: 0.5, breathiness: 0.2, brightness: 0.3, roughness: 0.05, vibratoRate: 6 },
    vibrato: 0,
  },
];

/** The menu's entries: the body's own tone first, then the presets. */
export type VoiceChoice = "default" | (typeof VOICE_PRESETS)[number]["id"];

const near = (a: number, b: number) => Math.abs(a - b) < 1e-6;
const sameTone = (s: PresetTone, t: PresetTone) => TONE_KEYS.every((k) => near(s[k], t[k]));

/** Which entry the voice is now: a preset, the body's own tone ("default"), or null (edited by hand). */
export function voiceChoiceOf(body: BodyMix, source: PresetTone, vibrato: number): VoiceChoice | null {
  for (const p of VOICE_PRESETS) if (sameBody(body, p.body) && sameTone(source, p.tone) && near(vibrato, p.vibrato)) return p.id;
  return sameTone(source, DEFAULT_TONE) && near(vibrato, DEFAULT_VIBRATO) ? "default" : null;
}

/**
 * Pick a voice: a preset sets its body (rebuilding the voice only if the body changes), its tone
 * and its vibrato; "default" puts the tone and vibrato back and keeps the body.
 */
export function chooseVoice(id: VoiceChoice): void {
  const p = VOICE_PRESETS.find((v) => v.id === id);
  const st = useStore.getState();
  if (p && !sameBody(st.body, p.body)) st.setBody(p.body);
  useStore.getState().setSource(p ? p.tone : DEFAULT_TONE);
  useSing.getState().setVibrato(p ? p.vibrato : DEFAULT_VIBRATO);
}
