# Feed counter

Which versions of Wormhole are in use and how often it is downloaded, without telling anyone apart. A
Cloudflare Worker answers the download server's addresses with the same redirects to GitHub that the
redirect rules gave, and adds one data point per request to Workers Analytics Engine (`src/index.ts`
says exactly what):

| Address | What is counted |
|---|---|
| `downloads.getwormhole.app/stable/latest.json`, `/preview/latest.json`, `/unstable/latest.json` (up to 1.5) | an update check: which feed, and the app's version |
| `downloads.getwormhole.app/stable/Wormhole-latest.dmg`, `-aarch64.dmg`, `-x86_64.dmg` | the website's download links (universal, Apple silicon, Intel): which one, and browser, bot, other or none, by user agent |

An update check says the app's name and version in its user agent (`Wormhole-Updater/1.7.0`) and
nothing else ([`crates/wormhole-core/src/update/http.rs`](../../crates/wormhole-core/src/update/http.rs);
a test in `update/tests.rs` lists every header). A copy checks at launch and every six hours while it
runs, and nothing tells two copies apart, so the counter has checks per version, not copies: which
versions are in use and how that moves. The nearest there is to a count of copies is GitHub's download
count of a release's update archive, which only the in-app updater fetches (`stats.ts` prints it).

No IP address, full user agent or other header is kept, and Workers Logs are off. Versions 1.4 to 1.6
said more with a check (channel, chip, macOS version, and a `Wormhole-Count` header on the first check
of a day and month); what they still send is not read.

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
the update archive is fetched only by the in-app updater, so its count is about how many copies updated.

## Tests

`bun test`
