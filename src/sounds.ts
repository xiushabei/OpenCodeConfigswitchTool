/*
 * Sound feedback policy and playback. Assets are bundled Kenney CC0 files
 * decoded once by sounds-assets; the live context starts on the first gesture.
 */

import { getSoundBuffer, loadSoundBank } from "./sounds-assets";
import type { SoundName } from "./sounds-assets";

const STORAGE_KEY = "omo-config-switcher.sound-enabled";
const CLICK_INTERVAL_MS = 90;
const MAX_CLICK_VOICES = 2;

const SOUND_GAIN: Readonly<Record<SoundName, number>> = {
  click: 0.22,
  success: 0.3,
  error: 0.3,
};

interface Playback {
  readonly buffer: AudioBuffer;
  readonly gain: number;
  readonly result: boolean;
}

interface ActiveSource {
  readonly source: AudioBufferSourceNode;
  readonly gain: GainNode;
  readonly result: boolean;
}

let context: AudioContext | null = null;
let audioUnavailable = false;
let warned = false;
let interacted = false;
let enabled = readPreference();
let lastClickAt = -Infinity;
let sources: ActiveSource[] = [];

function readPreference(): boolean {
  try {
    return window.localStorage.getItem(STORAGE_KEY) !== "false";
  } catch (error) {
    if (error instanceof DOMException) return true;
    throw error;
  }
}

function logAudioFailure(error: unknown): void {
  if (warned) {
    return;
  }
  warned = true;
  console.warn("操作音效不可用，已静默禁用。", error);
}

export function getSoundEnabled(): boolean {
  return enabled;
}

export function setSoundEnabled(next: boolean): void {
  enabled = next;
  try {
    window.localStorage.setItem(STORAGE_KEY, String(next));
  } catch (error) {
    if (!(error instanceof DOMException)) {
      throw error;
    }
  }
  if (!next && context !== null) stopSources(context.currentTime, false);
}

export function hasUserInteracted(): boolean {
  return interacted;
}

function ensureContext(): AudioContext | null {
  if (audioUnavailable) {
    return null;
  }
  if (typeof AudioContext === "undefined") {
    audioUnavailable = true;
    logAudioFailure(new Error("Web Audio is unavailable."));
    return null;
  }
  if (context === null) {
    try {
      context = new AudioContext();
    } catch (error) {
      audioUnavailable = true;
      if (!(error instanceof Error)) throw error;
      logAudioFailure(error);
      return null;
    }
  }
  return context;
}

function countClicks(): number {
  return sources.filter((entry) => !entry.result).length;
}

function stopSources(now: number, onlyResults: boolean): void {
  const keep: ActiveSource[] = [];
  for (const entry of sources) {
    if (onlyResults && !entry.result) {
      keep.push(entry);
      continue;
    }
    try {
      entry.gain.gain.cancelAndHoldAtTime(now);
      entry.gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.015);
      entry.source.stop(now + 0.03);
    } catch (error) {
      if (!(error instanceof DOMException)) throw error;
    }
  }
  sources = keep;
}

function startSource(ctx: AudioContext, playback: Playback): void {
  if (playback.result) stopSources(ctx.currentTime, true);
  const source = ctx.createBufferSource();
  const gain = ctx.createGain();
  source.buffer = playback.buffer;
  gain.gain.value = playback.gain;
  source.connect(gain);
  gain.connect(ctx.destination);
  const entry: ActiveSource = {
    source,
    gain,
    result: playback.result,
  };
  sources.push(entry);
  source.addEventListener(
    "ended",
    () => {
      source.disconnect();
      gain.disconnect();
      const index = sources.indexOf(entry);
      if (index >= 0) {
        sources.splice(index, 1);
      }
    },
    { once: true },
  );
  source.start();
}

function playSound(name: SoundName, result: boolean): boolean {
  if (!enabled || !interacted) {
    return false;
  }
  const buffer = getSoundBuffer(name);
  if (buffer === undefined) {
    return false;
  }
  if (!result && countClicks() >= MAX_CLICK_VOICES) {
    return false;
  }
  const ctx = ensureContext();
  if (ctx === null) {
    return false;
  }
  const playback: Playback = { buffer, gain: SOUND_GAIN[name], result };
  if (ctx.state === "suspended") {
    void ctx.resume().then(
      () => {
        if (enabled && interacted) startSource(ctx, playback);
      },
      (error: unknown) => {
        audioUnavailable = true;
        // no-excuse-ok: catch - audio is optional; report once and stay mute.
        logAudioFailure(error);
      },
    );
    return true;
  }
  startSource(ctx, playback);
  return true;
}

export function playClick(): void {
  const now = performance.now();
  if (now - lastClickAt >= CLICK_INTERVAL_MS && playSound("click", false)) {
    lastClickAt = now;
  }
}

export function playResult(tone: "success" | "error"): void {
  playSound(tone, true);
}

function isClickTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) {
    return false;
  }
  const element = target.closest("button, [role='tab']");
  if (!(element instanceof HTMLElement)) {
    return false;
  }
  return (
    element.dataset["sound"] !== "none" &&
    !(element instanceof HTMLButtonElement && element.disabled) &&
    element.getAttribute("aria-disabled") !== "true"
  );
}

function isChangeTarget(target: EventTarget | null): boolean {
  if (target instanceof HTMLInputElement) {
    return (
      target.dataset["sound"] !== "none" &&
      (target.type === "checkbox" || target.type === "radio")
    );
  }
  return target instanceof HTMLSelectElement;
}

function markInteraction(): void {
  if (interacted) {
    return;
  }
  interacted = true;
  if (enabled) {
    ensureContext();
  }
}

function closeAudio(): void {
  sources = [];
  if (context === null) {
    return;
  }
  const closing = context;
  context = null;
  void closing.close().catch((error: unknown) => {
    // no-excuse-ok: catch - teardown failure is non-fatal.
    logAudioFailure(error);
  });
}

export function installSoundFeedback(): () => void {
  loadSoundBank(logAudioFailure);
  const onGesture = (): void => {
    markInteraction();
  };
  const onClick = (event: MouseEvent): void => {
    if (isClickTarget(event.target)) {
      playClick();
    }
  };
  const onChange = (event: Event): void => {
    if (isChangeTarget(event.target)) {
      playClick();
    }
  };
  const onPageHide = (): void => {
    closeAudio();
  };

  document.addEventListener("pointerdown", onGesture, true);
  document.addEventListener("keydown", onGesture, true);
  document.addEventListener("click", onClick);
  document.addEventListener("change", onChange);
  window.addEventListener("pagehide", onPageHide);

  return () => {
    document.removeEventListener("pointerdown", onGesture, true);
    document.removeEventListener("keydown", onGesture, true);
    document.removeEventListener("click", onClick);
    document.removeEventListener("change", onChange);
    window.removeEventListener("pagehide", onPageHide);
    closeAudio();
  };
}
