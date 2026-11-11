/**
 * The download server's addresses, counted on the way to GitHub. It answers each with the
 * same 302 the redirect rules gave, and adds one data point to Workers Analytics Engine:
 *
 * - an update check (`/stable/latest.json`, `/preview/latest.json`, and `/unstable/latest.json`, which
 *   the Unstable channel reads up to 1.5): which feed, and the app's version, which its user agent
 *   names (`Wormhole-Updater/1.7.0`). The sum is how many checks each version made, not how many
 *   copies run: a copy checks at launch and every six hours, and nothing tells two copies apart.
 * - the website's download links (`/stable/Wormhole-latest.dmg`, universal, and `-aarch64`, `-x86_64`):
 *   which one, and whether a browser asked or a crawler, link preview or script did, by its user agent.
 *
 * Nothing else is kept: no IP address, no full user agent, no other header. Versions 1.4 to 1.6 said
 * more (the channel, the chip, the macOS version, and a `Wormhole-Count` header on the first check of
 * a day and month); what they still send is not read. Workers Logs stay off (wrangler.jsonc), so no
 * request is logged either.
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
  "/preview/latest.json": { kind: "feed", to: `${RELEASES}/download/preview/latest.json` },
  // The Unstable channel up to 1.5: the builds of main are Preview builds now, on the `preview` release.
  "/unstable/latest.json": { kind: "feed", to: `${RELEASES}/download/preview/latest.json` },
  "/stable/Wormhole-latest.dmg": { kind: "dmg", to: `${RELEASES}/latest/download/Wormhole-macos.dmg` },
  // From 1.4 a release also has a DMG per architecture.
  "/stable/Wormhole-latest-aarch64.dmg": { kind: "dmg", to: `${RELEASES}/latest/download/Wormhole-macos-aarch64.dmg` },
  "/stable/Wormhole-latest-x86_64.dmg": { kind: "dmg", to: `${RELEASES}/latest/download/Wormhole-macos-x86_64.dmg` },
};

/** `Wormhole-Updater/1.7.0`. What 1.4 to 1.6 put after the version, in brackets, is let through and not kept. */
const UPDATER = /^Wormhole-Updater\/([0-9A-Za-z.+-]{1,40})(?: \([^)]{0,80}\))?$/;

/** User agents that are not a person clicking a link. */
const BOT = /bot|crawl|spider|slurp|preview|facebookexternalhit|embed|scan|monitor|check|curl|wget|python|go-http|java\/|okhttp|libwww|httpclient|headless/i;

/** What one request adds to the counts, or `null` for a request that isn't counted. */
export function dataPoint(path: string, method: string, headers: Headers): DataPoint | null {
  const target = TARGETS[path];
  if (!target || method !== "GET") return null;
  const agent = headers.get("user-agent") ?? "";
  if (target.kind === "dmg") {
    const client = !agent ? "none" : BOT.test(agent) ? "bot" : agent.startsWith("Mozilla/") ? "browser" : "other";
    return { indexes: ["dmg"], blobs: ["dmg", path, "", client], doubles: [1] };
  }
  const version = UPDATER.exec(agent)?.[1];
  return { indexes: ["feed"], blobs: ["feed", path, version ?? "", version ? "app" : "other"], doubles: [1] };
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
