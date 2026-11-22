// Minimal embeddable player: one app, a canvas and key input, without the library UI.
// Built by webpack.config.player.ts into a standalone bundle exposing `window.WiePlayer`.
import { AudioPlayer } from "./midi";

export interface PlayerOptions {
  canvas: HTMLCanvasElement;
  /** Archive file name; its extension selects the platform (".jar" for J2ME). */
  filename: string;
  archive: Uint8Array;
  /** J2ME: application descriptor (JAD) attributes, over the jar manifest's; MIDlet.getAppProperty reads them. */
  jad?: Record<string, string>;
  /** J2ME: system properties, e.g. `wie.keyCodes` ("nokia" or "motorola"). */
  systemProperties?: Record<string, string>;
  /** The app finished preparing and shows its first screen. */
  onReady?: () => void;
  /** The app exited by itself (MIDlet.notifyDestroyed). */
  onExit?: () => void;
  onError?: (error: unknown) => void;
}

export interface PlayerSession {
  /** The app's id: its records live in the IndexedDB database `wie_<appId>`. */
  readonly appId: string;
  /** Key names: UP DOWN LEFT RIGHT OK LSK RSK CLR 0-9 * #. */
  key(key: string, pressed: boolean): void;
  releaseKeys(): void;
  suspend(suspended: boolean): void;
  stop(): void;
}

declare const __webpack_public_path__: string;
// MIDI files sit next to this script, not next to the page that embeds it.
(globalThis as { wieAssetBase?: string }).wieAssetBase = __webpack_public_path__;

const fontUrl = new URL("../../../assets/neodgm.ttf", import.meta.url);

export async function start(options: PlayerOptions): Promise<PlayerSession> {
  const [{ WieWeb, extractAppMetadata }, font] = await Promise.all([
    import("@pkg"),
    fetch(fontUrl).then(async response => {
      if (!response.ok) throw new Error(`Failed to load font: ${response.status}`);
      return new Uint8Array(await response.arrayBuffer());
    }),
  ]);

  const appId = extractAppMetadata(options.filename, options.archive).id;
  const audio = new AudioPlayer();
  let emulator: InstanceType<typeof WieWeb>;
  try {
    const j2me =
      options.jad || options.systemProperties
        ? JSON.stringify({ jad: options.jad, systemProperties: options.systemProperties })
        : undefined;
    emulator = new WieWeb(options.filename, options.archive, options.canvas, font, audio, undefined, j2me);
  } catch (error) {
    audio.dispose();
    throw error;
  }

  const keys = new Set<string>();
  let running = true;
  let preparing = true;
  let suspended = false;
  let frame = 0;

  const fail = (error: unknown) => {
    running = false;
    cancelAnimationFrame(frame);
    options.onError?.(error);
  };

  const update = () => {
    if (!running || suspended) return;
    try {
      emulator.update();
      if (emulator.is_exited()) {
        session.stop();
        options.onExit?.();
        return;
      }
      if (preparing && !emulator.is_preparing()) {
        preparing = false;
        options.onReady?.();
      }
      frame = requestAnimationFrame(update);
    } catch (error) {
      fail(error);
    }
  };

  const session: PlayerSession = {
    appId,
    key(key, pressed) {
      if (!running || preparing || suspended) return;
      try {
        if (pressed && !keys.has(key)) {
          emulator.key_down(key);
          keys.add(key);
        } else if (!pressed && keys.delete(key)) {
          emulator.key_up(key);
        }
      } catch (error) {
        fail(error);
      }
    },
    releaseKeys() {
      if (!running) return;
      for (const key of keys) emulator.key_up(key);
      keys.clear();
    },
    suspend(paused) {
      if (!running || paused === suspended) return;
      suspended = paused;
      session.releaseKeys();
      cancelAnimationFrame(frame);
      if (!suspended) frame = requestAnimationFrame(update);
    },
    stop() {
      if (!running) return;
      running = false;
      cancelAnimationFrame(frame);
      emulator.free();
    },
  };

  frame = requestAnimationFrame(update);
  return session;
}
