/**
 * The download server's addresses, counted on the way to GitHub. It answers each with the
 * same 302 the redirect rules gave, and adds one data point to Workers Analytics Engine:
 *
 * - an update check (`/stable/latest.json`, `/unstable/latest.json`): the app's version, and from
 *   1.4 its channel, chip and macOS major version (its user agent), and whether it is the copy's
 *   first check of a UTC day or month (`Wormhole-Count: day, month`). The flags summed are the
 *   copies running that day or month, with no ID: nothing tells two checks of one Mac apart.
 * - the website's download links (`/stable/Wormhole-latest.dmg`, universal, and `-aarch64`, `-x86_64`):
 *   which one, and whether a browser asked or a crawler, link preview or script did, by its user agent.
 *
 * Nothing else is kept: no IP address, no full user agent, no other header. Workers Logs stay off
 * (wrangler.jsonc), so no request is logged either.
 */

/** The binding wrangler.jsonc names; Analytics Engine's `writeDataPoint`, typed by hand. */
export interface Env {
  COUNTS: { writeDataPoint(point: DataPoint): void };
}

export interface DataPoint {
  indexes: string[];
  blobs: string[];
  doubles: number[];
}

const RELEASES = "https://github.com/wormhole-terminal/wormhole/releases";

/** Each address and where it goes, as the Cloudflare redirect rules sent them. */
export const TARGETS: Record<string, { kind: "feed" | "dmg"; to: string }> = {
  "/stable/latest.json": { kind: "feed", to: `${RELEASES}/latest/download/latest.json` },
  "/unstable/latest.json": { kind: "feed", to: `${RELEASES}/download/unstable/latest.json` },
  "/stable/Wormhole-latest.dmg": { kind: "dmg", to: `${RELEASES}/latest/download/Wormhole-macos.dmg` },
  // From 1.4 a release also has a DMG per architecture.
  "/stable/Wormhole-latest-aarch64.dmg": { kind: "dmg", to: `${RELEASES}/latest/download/Wormhole-macos-aarch64.dmg` },
  "/stable/Wormhole-latest-x86_64.dmg": { kind: "dmg", to: `${RELEASES}/latest/download/Wormhole-macos-x86_64.dmg` },
};

/** `Wormhole-Updater/1.4.0 (stable; aarch64; macOS 26)`; before 1.4 the version alone. */
const UPDATER = /^Wormhole-Updater\/([0-9A-Za-z.+-]{1,40})(?: \((stable|unstable); (aarch64|x86_64|unknown); macOS (\d{1,3}|unknown)\))?$/;

/** User agents that are not a person clicking a link. */
const BOT = /bot|crawl|spider|slurp|preview|facebookexternalhit|embed|scan|monitor|check|curl|wget|python|go-http|java\/|okhttp|libwww|httpclient|headless/i;

/** What one request adds to the counts, or `null` for a request that isn't counted. */
export function dataPoint(path: string, method: string, headers: Headers): DataPoint | null {
  const target = TARGETS[path];
  if (!target || method !== "GET") return null;
  const agent = headers.get("user-agent") ?? "";
  if (target.kind === "dmg") {
    const client = !agent ? "none" : BOT.test(agent) ? "bot" : agent.startsWith("Mozilla/") ? "browser" : "other";
    return { indexes: ["dmg"], blobs: ["dmg", path, "", "", "", "", client], doubles: [1, 0, 0] };
  }
  const m = UPDATER.exec(agent);
  if (!m) {
    return { indexes: ["feed"], blobs: ["feed", path, "", "", "", "", "other"], doubles: [1, 0, 0] };
  }
  const [, version, channel = "", arch = "", macos = ""] = m;
  const flags = new Set(
    (headers.get("wormhole-count") ?? "")
      .split(",")
      .map((f) => f.trim())
      .filter(Boolean),
  );
  // Only an app that says how it runs sends flags; anything else claiming them isn't the app.
  const counted = channel !== "";
  return {
    indexes: ["feed"],
    blobs: ["feed", path, version, channel, arch, macos, "app"],
    doubles: [1, counted && flags.has("day") ? 1 : 0, counted && flags.has("month") ? 1 : 0],
  };
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const target = TARGETS[url.pathname];
    // The routes name only the three addresses; anything else goes on to where it went before.
    if (!target) return fetch(request);
    const point = dataPoint(url.pathname, request.method, request.headers);
    if (point) env.COUNTS.writeDataPoint(point);
    return new Response(null, {
      status: 302,
      headers: { location: target.to, "cache-control": "no-store" },
    });
  },
};
