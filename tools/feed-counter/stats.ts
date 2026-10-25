/**
 * How many people use Wormhole, from the two places that know:
 *
 *   bun stats.ts               GitHub's download counts per release (no token needed)
 *   bun stats.ts --days 30     and, with CF_ACCOUNT_ID and CF_API_TOKEN set (a token with Account
 *                              Analytics: Read), the counter's numbers (src/index.ts)
 *
 * GitHub counts every fetch of a file, by people and bots alike: the DMG is an upper bound. The update
 * archive is fetched by the in-app updater only, so its count is about how many copies updated.
 * Analytics Engine keeps three months.
 */

const REPO = "wormhole-terminal/wormhole";
const DATASET = "wormhole_counts";

const days = Number(process.argv[process.argv.indexOf("--days") + 1]) || 14;

interface Asset {
  name: string;
  download_count: number;
}

async function github(): Promise<void> {
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=20`, {
    headers: { accept: "application/vnd.github+json" },
  });
  if (!res.ok) throw new Error(`GitHub: ${res.status} ${res.statusText}`);
  const releases = (await res.json()) as { tag_name: string; assets: Asset[] }[];
  console.log("GitHub downloads (people and bots alike)");
  console.log("  release        dmg  update archive");
  for (const r of releases.filter((r) => r.tag_name.startsWith("v"))) {
    const sum = (test: (n: string) => boolean) =>
      r.assets.filter((a) => test(a.name)).reduce((n, a) => n + a.download_count, 0);
    const dmg = sum((n) => n.endsWith(".dmg"));
    const archive = sum((n) => n.endsWith(".app.tar.gz"));
    console.log(`  ${r.tag_name.padEnd(12)} ${String(dmg).padStart(5)}  ${String(archive).padStart(14)}`);
  }
}

async function sql(query: string): Promise<Record<string, string | number>[]> {
  const account = process.env.CF_ACCOUNT_ID;
  const token = process.env.CF_API_TOKEN;
  const res = await fetch(`https://api.cloudflare.com/client/v4/accounts/${account}/analytics_engine/sql`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}` },
    body: query,
  });
  if (!res.ok) throw new Error(`Analytics Engine: ${res.status} ${await res.text()}`);
  return ((await res.json()) as { data: Record<string, string | number>[] }).data;
}

function table(title: string, rows: Record<string, string | number>[]): void {
  console.log(`\n${title}`);
  if (!rows.length) {
    console.log("  (nothing yet)");
    return;
  }
  const keys = Object.keys(rows[0]);
  console.log(`  ${keys.join("  ")}`);
  for (const row of rows) console.log(`  ${keys.map((k) => String(row[k])).join("  ")}`);
}

async function counter(): Promise<void> {
  if (!process.env.CF_ACCOUNT_ID || !process.env.CF_API_TOKEN) {
    console.log("\n(set CF_ACCOUNT_ID and CF_API_TOKEN for the counter's numbers)");
    return;
  }
  const since = `timestamp > NOW() - INTERVAL '${days}' DAY`;
  const day = "formatDateTime(toStartOfInterval(timestamp, INTERVAL '1' DAY), '%Y-%m-%d')";
  table(
    `Running copies per day (1.4 and later), update checks, website downloads — last ${days} days`,
    await sql(`SELECT ${day} AS day,
        SUM(IF(blob1 = 'feed', _sample_interval * double2, 0)) AS running,
        SUM(IF(blob1 = 'feed', _sample_interval * double1, 0)) AS checks,
        SUM(IF(blob1 = 'dmg' AND blob7 = 'browser', _sample_interval * double1, 0)) AS dmg_browsers,
        SUM(IF(blob1 = 'dmg' AND blob7 != 'browser', _sample_interval * double1, 0)) AS dmg_other
      FROM ${DATASET} WHERE ${since} GROUP BY day ORDER BY day`),
  );
  table(
    "Running copies per month",
    await sql(`SELECT formatDateTime(toStartOfInterval(timestamp, INTERVAL '1' MONTH), '%Y-%m') AS month,
        SUM(_sample_interval * double3) AS running
      FROM ${DATASET} WHERE blob1 = 'feed' GROUP BY month ORDER BY month`),
  );
  table(
    "Today's running copies by version, channel, chip and macOS",
    await sql(`SELECT blob3 AS version, blob4 AS channel, blob5 AS chip, blob6 AS macos,
        SUM(_sample_interval * double2) AS running
      FROM ${DATASET} WHERE blob1 = 'feed' AND timestamp > NOW() - INTERVAL '1' DAY AND double2 > 0
      GROUP BY version, channel, chip, macos ORDER BY running DESC`),
  );
  table(
    "Checks from versions before 1.4 (no daily flag: roughly 2–4 checks a day each)",
    await sql(`SELECT blob3 AS version, SUM(_sample_interval * double1) AS checks
      FROM ${DATASET} WHERE blob1 = 'feed' AND blob7 = 'app' AND blob4 = '' AND ${since}
      GROUP BY version ORDER BY checks DESC`),
  );
}

await github();
await counter();
