# Feed counter

How many people use Wormhole, counted without telling anyone apart. A Cloudflare Worker answers the
download server's three addresses with the same redirects to GitHub that the redirect rules gave, and
adds one data point per request to Workers Analytics Engine (`src/index.ts` says exactly what):

| Address | What is counted |
|---|---|
| `downloads.getwormhole.app/stable/latest.json`, `/preview/latest.json`, `/unstable/latest.json` (up to 1.5) | an update check: version, and from 1.4 channel, chip, macOS major version, and the daily and monthly flag |
| `downloads.getwormhole.app/stable/Wormhole-latest.dmg`, `-aarch64.dmg`, `-x86_64.dmg` | the website's download links (universal, Apple silicon, Intel; the last two from 1.4): which one, and browser, bot, other or none, by user agent |

From 1.4 the app sends `Wormhole-Count: day` on its first check of a UTC day and `month` on its
first of a UTC month ([`crates/wormhole-core/src/update/count.rs`](../../crates/wormhole-core/src/update/count.rs)), starting a day after a
version that counts first runs, unless Settings → Updates → "Count this Mac anonymously" is off.
The day's `day` flags are the copies that ran that day; nothing links two checks of one Mac. No IP address, full user agent or other header is kept,
and Workers Logs are off.

## Set up (once)

1. `bun install`, then `bunx wrangler login` (the Cloudflare account with getwormhole.app).
2. Cloudflare dashboard → getwormhole.app → Rules → Redirect Rules: turn off "Update feed to
   GitHub", "Latest DMG to GitHub" and the preview and unstable feeds' rules. Redirect rules run before Workers,
   so while they are on the Worker never sees a request.
3. `bun run deploy`. The first deploy makes the `wormhole_counts` dataset.
4. Check: `curl -sI https://downloads.getwormhole.app/stable/latest.json` is a 302 to GitHub, and the
   app's Settings → Updates → Check for updates still works.

To go back, turn the rules on again and delete the Worker's routes (or the Worker).

## Read the numbers

```sh
bun stats.ts                       # GitHub's download counts per release
CF_ACCOUNT_ID=… CF_API_TOKEN=… bun stats.ts --days 30   # and the counter's
```

The token needs Account → Account Analytics → Read. Analytics Engine keeps three months; check its
current free-plan limits on Cloudflare's pricing page before relying on them.

GitHub counts every fetch of a file, by people and bots alike, so its DMG count is an upper bound;
the update archive is fetched only by the in-app updater.

## Tests

`bun test`
