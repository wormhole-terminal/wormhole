import { describe, expect, test } from "bun:test";
import worker, { type DataPoint, dataPoint } from "./index.ts";

const headers = (h: Record<string, string>) => new Headers(h);

describe("update checks", () => {
  test("a check is its feed and the app's version", () => {
    const p = dataPoint("/stable/latest.json", "GET", headers({ "user-agent": "Wormhole-Updater/1.7.0" }));
    expect(p).toEqual({ indexes: ["feed"], blobs: ["feed", "/stable/latest.json", "1.7.0", "app"], doubles: [1] });
  });

  test("a preview build is counted on its own feed", () => {
    const p = dataPoint("/preview/latest.json", "GET", headers({ "user-agent": "Wormhole-Updater/1.7.1-preview.6" }));
    expect(p?.blobs).toEqual(["feed", "/preview/latest.json", "1.7.1-preview.6", "app"]);
  });

  test("what 1.4 to 1.6 said besides the version is not kept", () => {
    const p = dataPoint(
      "/stable/latest.json",
      "GET",
      headers({ "user-agent": "Wormhole-Updater/1.4.0 (stable; aarch64; macOS 26)", "wormhole-count": "day, month" }),
    );
    expect(p).toEqual({ indexes: ["feed"], blobs: ["feed", "/stable/latest.json", "1.4.0", "app"], doubles: [1] });
    expect(JSON.stringify(p)).not.toMatch(/aarch64|macOS|stable;|day|month/);
  });

  test("anything else asking for the feed is counted apart, without its user agent", () => {
    const p = dataPoint("/stable/latest.json", "GET", headers({ "user-agent": "curl/8.7.1" }));
    expect(p?.blobs).toEqual(["feed", "/stable/latest.json", "", "other"]);
  });
});

describe("the download link", () => {
  const client = (agent: string) => dataPoint("/stable/Wormhole-latest.dmg", "GET", headers({ "user-agent": agent }))?.blobs[3];

  test("tells browsers from bots", () => {
    expect(client("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari/605.1.15")).toBe("browser");
    expect(client("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)")).toBe("bot");
    expect(client("Slackbot-LinkExpanding 1.0 (+https://api.slack.com/robots)")).toBe("bot");
    expect(client("curl/8.7.1")).toBe("bot");
    expect(client("Transmit/5")).toBe("other");
    expect(dataPoint("/stable/Wormhole-latest.dmg", "GET", headers({}))?.blobs[3]).toBe("none");
  });

  test("a HEAD request is redirected but not counted", () => {
    expect(dataPoint("/stable/Wormhole-latest.dmg", "HEAD", headers({ "user-agent": "Mozilla/5.0" }))).toBeNull();
  });
});

describe("the worker", () => {
  test("redirects as the rules did and writes one point, nothing from the address", async () => {
    const points: DataPoint[] = [];
    const env = { COUNTS: { writeDataPoint: (p: DataPoint) => points.push(p) } };
    const res = await worker.fetch(
      new Request("https://downloads.getwormhole.app/stable/latest.json", {
        headers: { "user-agent": "Wormhole-Updater/1.7.0", "cf-connecting-ip": "203.0.113.7", cookie: "id=42" },
      }),
      env,
    );
    expect(res.status).toBe(302);
    expect(res.headers.get("location")).toBe(
      "https://github.com/wormhole-terminal/wormhole/releases/latest/download/latest.json",
    );
    expect(points).toHaveLength(1);
    expect(points).toEqual([{ indexes: ["feed"], blobs: ["feed", "/stable/latest.json", "1.7.0", "app"], doubles: [1] }]);
  });

  test("every address goes where its rule sent it", async () => {
    const env = { COUNTS: { writeDataPoint: () => {} } };
    const at = async (path: string) =>
      (await worker.fetch(new Request(`https://downloads.getwormhole.app${path}`), env)).headers.get("location");
    expect(await at("/preview/latest.json")).toBe(
      "https://github.com/wormhole-terminal/wormhole/releases/download/preview/latest.json",
    );
    // The Unstable channel up to 1.5 reads the same builds.
    expect(await at("/unstable/latest.json")).toBe(
      "https://github.com/wormhole-terminal/wormhole/releases/download/preview/latest.json",
    );
    expect(await at("/stable/Wormhole-latest.dmg")).toBe(
      "https://github.com/wormhole-terminal/wormhole/releases/latest/download/Wormhole-macos.dmg",
    );
    expect(await at("/stable/Wormhole-latest-aarch64.dmg")).toBe(
      "https://github.com/wormhole-terminal/wormhole/releases/latest/download/Wormhole-macos-aarch64.dmg",
    );
    expect(await at("/stable/Wormhole-latest-x86_64.dmg")).toBe(
      "https://github.com/wormhole-terminal/wormhole/releases/latest/download/Wormhole-macos-x86_64.dmg",
    );
  });
});
