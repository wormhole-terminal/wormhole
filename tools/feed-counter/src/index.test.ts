import { describe, expect, test } from "bun:test";
import worker, { type DataPoint, dataPoint } from "./index.ts";

const headers = (h: Record<string, string>) => new Headers(h);

describe("update checks", () => {
  test("a 1.4 check says how it runs and what it is the first of", () => {
    const p = dataPoint(
      "/stable/latest.json",
      "GET",
      headers({ "user-agent": "Wormhole-Updater/1.4.0 (stable; aarch64; macOS 26)", "wormhole-count": "day, month" }),
    );
    expect(p).toEqual({
      indexes: ["feed"],
      blobs: ["feed", "/stable/latest.json", "1.4.0", "stable", "aarch64", "26", "app"],
      doubles: [1, 1, 1],
    });
  });

  test("a check that is the first of nothing counts as a check only", () => {
    const p = dataPoint(
      "/unstable/latest.json",
      "GET",
      headers({ "user-agent": "Wormhole-Updater/1.4.1-dev.3 (unstable; x86_64; macOS unknown)" }),
    );
    expect(p?.blobs.slice(2, 6)).toEqual(["1.4.1-dev.3", "unstable", "x86_64", "unknown"]);
    expect(p?.doubles).toEqual([1, 0, 0]);
  });

  test("an older app has its version only, and flags from it are not believed", () => {
    const p = dataPoint(
      "/stable/latest.json",
      "GET",
      headers({ "user-agent": "Wormhole-Updater/1.3.0", "wormhole-count": "day" }),
    );
    expect(p?.blobs.slice(2, 7)).toEqual(["1.3.0", "", "", "", "app"]);
    expect(p?.doubles).toEqual([1, 0, 0]);
  });

  test("anything else asking for the feed is counted apart", () => {
    const p = dataPoint("/stable/latest.json", "GET", headers({ "user-agent": "curl/8.7.1", "wormhole-count": "day" }));
    expect(p?.blobs).toEqual(["feed", "/stable/latest.json", "", "", "", "", "other"]);
    expect(p?.doubles).toEqual([1, 0, 0]);
  });
});

describe("the download link", () => {
  const client = (agent: string) => dataPoint("/stable/Wormhole-latest.dmg", "GET", headers({ "user-agent": agent }))?.blobs[6];

  test("tells browsers from bots", () => {
    expect(client("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari/605.1.15")).toBe("browser");
    expect(client("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)")).toBe("bot");
    expect(client("Slackbot-LinkExpanding 1.0 (+https://api.slack.com/robots)")).toBe("bot");
    expect(client("curl/8.7.1")).toBe("bot");
    expect(client("Transmit/5")).toBe("other");
    expect(dataPoint("/stable/Wormhole-latest.dmg", "GET", headers({}))?.blobs[6]).toBe("none");
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
        headers: { "user-agent": "Wormhole-Updater/1.4.0 (stable; aarch64; macOS 26)", "cf-connecting-ip": "203.0.113.7" },
      }),
      env,
    );
    expect(res.status).toBe(302);
    expect(res.headers.get("location")).toBe(
      "https://github.com/wormhole-terminal/wormhole/releases/latest/download/latest.json",
    );
    expect(points).toHaveLength(1);
    expect(JSON.stringify(points)).not.toContain("203.0.113.7");
  });

  test("every address goes where its rule sent it", async () => {
    const env = { COUNTS: { writeDataPoint: () => {} } };
    const at = async (path: string) =>
      (await worker.fetch(new Request(`https://downloads.getwormhole.app${path}`), env)).headers.get("location");
    expect(await at("/unstable/latest.json")).toBe(
      "https://github.com/wormhole-terminal/wormhole/releases/download/unstable/latest.json",
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
