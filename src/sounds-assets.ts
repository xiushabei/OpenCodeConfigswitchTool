/*
 * Bundled Kenney Interface Sounds (CC0). Files are local Vite assets, fetched
 * once and decoded for reuse across AudioContexts.
 */

export const SOUND_NAMES = ["click", "success", "error"] as const;

export type SoundName = (typeof SOUND_NAMES)[number];

export type FailureReporter = (error: unknown) => void;

const SOUND_URLS: Readonly<Record<SoundName, string>> = {
  click: new URL("./assets/sounds/click.ogg", import.meta.url).href,
  success: new URL("./assets/sounds/success.ogg", import.meta.url).href,
  error: new URL("./assets/sounds/error.ogg", import.meta.url).href,
};

const buffers = new Map<SoundName, AudioBuffer>();
let loading = false;

function createDecoder(report: FailureReporter): BaseAudioContext | null {
  try {
    return new OfflineAudioContext(1, 1, 48000);
  } catch (error) {
    if (!(error instanceof Error)) {
      throw error;
    }
    report(error);
    return null;
  }
}

async function decodeAsset(
  name: SoundName,
  url: string,
  decoder: BaseAudioContext,
): Promise<void> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`音效资源加载失败：${url}（HTTP ${response.status}）`);
  }
  const data = await response.arrayBuffer();
  if (data.byteLength === 0) {
    throw new Error(`音效资源为空：${url}`);
  }
  buffers.set(name, await decoder.decodeAudioData(data));
}

export function loadSoundBank(report: FailureReporter): void {
  if (loading) {
    return;
  }
  loading = true;
  const decoder = createDecoder(report);
  if (decoder === null) {
    return;
  }
  for (const name of SOUND_NAMES) {
    void decodeAsset(name, SOUND_URLS[name], decoder).catch(
      // no-excuse-ok: catch - audio is optional; report once and continue mute.
      (error: unknown) => {
        report(error);
      },
    );
  }
}

export function getSoundBuffer(name: SoundName): AudioBuffer | undefined {
  return buffers.get(name);
}
