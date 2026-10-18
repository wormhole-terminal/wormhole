//! The steps of `schema::MIGRATIONS`, one constant each, in order. Never edit a step that has
//! shipped: a database out there may already have run it (`tests/migrations.rs` and the tests
//! beside it freeze the layout each step starts from).

/// Version 6 to 7: no tabs. Every pane becomes a session of its own, and a tab with several panes
/// becomes a split.
///
/// Version 6: a session has tabs, a tab has panes, and a `tabs` row is a pane (`tab_id` groups a
/// tab's rows, `position` orders them tab by tab, the tab's layout is on its first row);
/// `scrollback.tab_id` names the pane's row.
///
/// Version 7: a session is one terminal. Its `tabs` row has the session's id (so scrollback, keyed
/// by that row, needs no change but its key) and nothing of the old grouping any more (`tab_id` is
/// the row's id, `position` 0, `layout` NULL, as is `sessions.active_tab` 0: columns kept unused
/// until a later step drops them). Panes that shared a tab share a `splits` row, which holds the
/// tab's layout, and name it in `sessions.split_id`.
///
/// What goes where, per old session S (its rows are its panes in the order the app showed them: tab
/// by tab, a tab's panes in their order):
/// - The first pane's row is S's own, with its name, notes, tasks and flags, and takes S's id (S
///   is what notes sync and the rest of the data are keyed by). Its pane id is then rewritten to S,
///   in `tabs`, `scrollback` and the layouts. S is a pane id of its own session only when that
///   pane has been moved out of the first tab since (`move_pane_to_new_tab`) and not closed:
///   then the two ids swap, so nothing collides.
/// - Every other pane becomes a new session with its own pane id, right after the one before it,
///   unnamed and with the `pinned` flag of S: pinning one member of a split pins the whole split.
///   Several tabs make adjacent rows, one split for each tab that has more than one pane.
/// - Positions are renumbered per window (the old sessions' order, then the panes' order).
/// - `windows.active` followed the selected session; it now names the row of the focused pane of
///   its selected tab (the layout's `focused`, else the tab's first pane).
/// - Split ids are one past everything else's, so ids stay unique across sessions, panes and
///   splits (`Store::next_ids` counts the splits too).
///
/// A layout is rewritten as text: the leaf, `focused` and `zoomed` of the app's pane tree JSON as
/// serde writes them (`"leaf":7}`, `"focused":7,`, `"zoomed":7}`). Should it ever not match, the
/// app fits a layout to the panes it finds and puts a pane it doesn't know to the right of the
/// others, so the worst that comes of it is an arrangement lost, never a terminal.
///
/// The foreign key from `scrollback` to `tabs` is only checked at the end: ids move in two
/// phases (to the negative of the old one and then to the new one, as SQLite checks a key at each
/// row).
pub(super) const V7_EVERY_TERMINAL_A_SESSION: &str = r#"
    PRAGMA defer_foreign_keys = ON;

    ALTER TABLE sessions ADD COLUMN split_id INTEGER;
    CREATE TABLE splits (
        id INTEGER PRIMARY KEY,
        window_id INTEGER NOT NULL,
        layout TEXT
    );
    CREATE INDEX splits_window ON splits (window_id);

    -- A session always has a pane (what `repair` makes sure of on every open).
    INSERT INTO tabs (id, session_id, tab_id, position)
        SELECT id, id, id, 0 FROM sessions WHERE id NOT IN (SELECT session_id FROM tabs);

    -- One row per pane: where it goes and what it becomes.
    CREATE TEMP TABLE v7_rows AS
    WITH located AS (
        SELECT t.id AS old_id, t.session_id AS sid, t.tab_id AS tab_key, t.position AS pos,
               s.window_id AS wid, s.position AS spos, s.pinned AS pinned, s.active_tab AS active_tab,
               MIN(t.position) OVER tab AS tab_start,
               COUNT(*) OVER tab AS tab_size,
               FIRST_VALUE(t.layout) OVER (PARTITION BY t.session_id, t.tab_id
                                           ORDER BY t.layout IS NULL, t.position, t.id) AS tab_layout
        FROM tabs t JOIN sessions s ON s.id = t.session_id
        WINDOW tab AS (PARTITION BY t.session_id, t.tab_id)
    ), ordered AS (
        SELECT *,
               ROW_NUMBER() OVER (PARTITION BY sid ORDER BY tab_start, tab_key, pos, old_id) AS ord,
               DENSE_RANK() OVER (PARTITION BY sid ORDER BY tab_start, tab_key) - 1 AS tab_ix
        FROM located
    ), placed AS (
        SELECT *,
               FIRST_VALUE(old_id) OVER (PARTITION BY sid ORDER BY ord) AS first_id,
               ROW_NUMBER() OVER (PARTITION BY wid ORDER BY spos, sid, ord) - 1 AS new_pos,
               DENSE_RANK() OVER (PARTITION BY wid ORDER BY spos, sid) - 1 AS old_ix,
               MAX(tab_ix) OVER (PARTITION BY sid) AS last_tab,
               CASE WHEN json_valid(tab_layout) THEN json_extract(tab_layout, '$.focused') END AS focused
        FROM ordered
    ), ranked AS (
        SELECT *,
               ROW_NUMBER() OVER (PARTITION BY sid, tab_ix ORDER BY old_id IS NOT focused, ord) AS focus_rank
        FROM placed
    )
    SELECT old_id, sid, tab_key, wid, pinned, tab_ix, tab_size, tab_layout, ord, first_id, new_pos, old_ix,
           CASE WHEN ord = 1 THEN sid WHEN old_id = sid THEN first_id ELSE old_id END AS new_id,
           focus_rank = 1 AND tab_ix = MIN(MAX(active_tab, 0), last_tab) AS picked
    FROM ranked;

    -- One split per tab with more than one pane.
    CREATE TEMP TABLE v7_splits AS
    SELECT sid, tab_key, wid, layout, first_id,
           (SELECT COALESCE(MAX(m), 0) FROM (SELECT MAX(id) AS m FROM sessions
                                             UNION ALL SELECT MAX(id) FROM tabs
                                             UNION ALL SELECT MAX(tab_id) FROM tabs))
               + ROW_NUMBER() OVER (ORDER BY wid, sid, tab_ix) AS split_id
    FROM (SELECT sid, tab_key, MIN(wid) AS wid, MIN(tab_ix) AS tab_ix, MIN(tab_layout) AS layout,
                 MIN(first_id) AS first_id
          FROM v7_rows WHERE tab_size > 1 GROUP BY sid, tab_key);

    -- The panes whose id changes (a session's first pane, and the pane that held the session's id):
    -- in the layouts the old id goes through a marker, `-<first pane's id>`, so a swap works.
    UPDATE v7_splits SET layout =
        REPLACE(REPLACE(REPLACE(layout,
            '"leaf":' || first_id || '}',    '"leaf":-' || first_id || '}'),
            '"focused":' || first_id || ',', '"focused":-' || first_id || ','),
            '"zoomed":' || first_id || '}',  '"zoomed":-' || first_id || '}')
        WHERE first_id <> sid AND layout IS NOT NULL;
    UPDATE v7_splits SET layout =
        REPLACE(REPLACE(REPLACE(layout,
            '"leaf":' || sid || '}',    '"leaf":' || first_id || '}'),
            '"focused":' || sid || ',', '"focused":' || first_id || ','),
            '"zoomed":' || sid || '}',  '"zoomed":' || first_id || '}')
        WHERE first_id <> sid AND layout IS NOT NULL;
    UPDATE v7_splits SET layout =
        REPLACE(REPLACE(REPLACE(layout,
            '"leaf":-' || first_id || '}',    '"leaf":' || sid || '}'),
            '"focused":-' || first_id || ',', '"focused":' || sid || ','),
            '"zoomed":-' || first_id || '}',  '"zoomed":' || sid || '}')
        WHERE first_id <> sid AND layout IS NOT NULL;

    -- The selected session was an index; it is the focused pane's row now.
    UPDATE windows SET active = COALESCE(
        (SELECT r.new_pos FROM v7_rows r
         WHERE r.wid = windows.id AND r.old_ix = MAX(windows.active, 0) AND r.picked),
        (SELECT r.new_pos FROM v7_rows r WHERE r.wid = windows.id AND r.picked ORDER BY r.old_ix DESC LIMIT 1),
        active);

    -- The panes' rows take their new ids: scrollback follows its row.
    UPDATE scrollback SET tab_id = -tab_id
        WHERE tab_id IN (SELECT old_id FROM v7_rows WHERE new_id <> old_id);
    UPDATE tabs SET id = -id WHERE id IN (SELECT old_id FROM v7_rows WHERE new_id <> old_id);
    UPDATE scrollback SET tab_id = (SELECT new_id FROM v7_rows WHERE old_id = -scrollback.tab_id)
        WHERE tab_id < 0;
    UPDATE tabs SET id = (SELECT new_id FROM v7_rows WHERE old_id = -tabs.id) WHERE id < 0;
    UPDATE tabs SET session_id = id, tab_id = id, position = 0, layout = NULL
        WHERE id IN (SELECT new_id FROM v7_rows);

    -- The sessions: the old ones keep their row and move, a pane after the first is a new one.
    UPDATE sessions SET active_tab = 0,
        position = (SELECT r.new_pos FROM v7_rows r WHERE r.sid = sessions.id AND r.ord = 1),
        split_id = (SELECT sp.split_id FROM v7_rows r
                    JOIN v7_splits sp ON sp.sid = r.sid AND sp.tab_key = r.tab_key
                    WHERE r.sid = sessions.id AND r.ord = 1)
        WHERE id IN (SELECT sid FROM v7_rows);
    INSERT INTO sessions (id, window_id, position, pinned, split_id)
        SELECT r.new_id, r.wid, r.new_pos, r.pinned, sp.split_id
        FROM v7_rows r LEFT JOIN v7_splits sp ON sp.sid = r.sid AND sp.tab_key = r.tab_key
        WHERE r.ord > 1;
    INSERT INTO splits (id, window_id, layout) SELECT split_id, wid, layout FROM v7_splits;

    DROP TABLE v7_rows;
    DROP TABLE v7_splits;
"#;

/// Version 7 to 8: a terminal may run under a session host. `tabs.host_id` holds the host's
/// 12-character id (random, so a recovered database can't hand one host to the wrong terminal);
/// NULL, as every existing row gets it, is a terminal in the app's own process.
pub(super) const V8_TERMINAL_HOST: &str = "ALTER TABLE tabs ADD COLUMN host_id TEXT;";
