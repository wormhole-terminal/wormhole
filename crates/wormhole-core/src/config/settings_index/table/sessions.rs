//! The Sessions and Git sections: the sidebar's terminals and rows, and the Changes panel and diff view.

use super::{literal_entry, literal_group};
use crate::config::FeatureId;
use crate::config::settings_index::{Group, Word};

pub(super) const SESSIONS: &[Group] = &[
    literal_group("", &[literal_entry("sidebar-preview", "Live preview", "", "preview sidebar", &[])]),
    literal_group(
        "Terminals in",
        &[literal_entry(
            "terminals-in",
            "Show terminals in",
            "Sidebar: the list on the left; Tabs: a tab bar instead, with the groups as sets of tabs",
            "tabs tab bar sidebar list layout",
            &["terminals_in"],
        )],
    ),
    Group {
        title: Word::Literal("Terminals list"),
        blurb: Word::Literal(""),
        aside: Word::Literal(""),
        entries: &[
            literal_entry(
                "density",
                "Density",
                "How much room each terminal takes, in the sidebar and the tab bar: Regular rows are 48 pt with a second line, Compact 32, Dense 26",
                "density compact dense regular comfortable rows size",
                &["density"],
            ),
            literal_entry(
                "cards",
                "Show groups as cards",
                "Off: groups are a head and their rows, with no card around them",
                "cards groups card background border",
                &["show_groups_as_cards"],
            ),
            literal_entry(
                "fade-group-colors",
                "Fade group colours",
                "On: a group's colour fades out across its card; off, it fills the card evenly",
                "fade gradient group colour color card wash",
                &["fade_group_colors"],
            ),
            literal_entry(
                "show-group-folders",
                "Show group folders",
                "The folder a group's new terminals start in, under its name",
                "group folder path directory name head show hide",
                &["show_group_folders"],
            ),
            literal_entry(
                "sidebar-numbers",
                "⌘1–⌘9 numbers",
                "Stay with the terminal: dealt down the list, then kept (a new terminal gets the next one); Follow the list: the first nine rows you can see",
                "command shortcuts numbers stay follow list stable classic",
                &["sidebar_numbers"],
            ),
            literal_entry(
                "cmd-hints",
                "Show ⌘1–⌘9 while ⌘ is held",
                "Keycaps on the first nine terminals after 250 ms; the shortcuts work either way",
                "command keycaps shortcuts numbers hints",
                &["sidebar_cmd_hints"],
            ),
        ],
    },
    literal_group(
        "Row details",
        &[
            literal_entry(
                "sidebar-changes",
                "Changed files",
                "±N uncommitted changes in the terminal's row",
                "git changes count",
                &["sidebar_changes"],
            ),
            literal_entry(
                "sidebar-dot",
                "Status dot",
                "Working, needs you, finished or idle, as the state round every terminal's status tile",
                "status glyph agent",
                &["sidebar_status_dot"],
            ),
            literal_entry(
                "sidebar-notes",
                "Notes and tasks",
                "Note icon with the number of notes and open tasks",
                "notes tasks next steps count",
                &["sidebar_notes"],
            )
            .of(FeatureId::Notes),
        ],
    ),
];

pub(super) const GIT: &[Group] = &[
    literal_group(
        "Changes panel",
        &[
            literal_entry(
                "changes-view",
                "File view",
                "A flat list, or folders as a tree",
                "folders files list tree",
                &["changes_tree"],
            ),
            literal_entry(
                "group-untracked",
                "Group untracked files separately",
                "New files get their own section under the changes",
                "untracked new files",
                &["group_untracked"],
            ),
        ],
    ),
    literal_group(
        "Diff view",
        &[
            literal_entry(
                "diff-layout",
                "Default layout",
                "Split keeps old and new side by side, even for a new or deleted file. The diff's own \
                 toolbar switches one diff",
                "split unified side by side",
                &["diff_split"],
            ),
            literal_entry(
                "diff-wrap",
                "Wrap long lines",
                "Otherwise the diff scrolls sideways",
                "wrap",
                &["diff_wrap"],
            ),
            literal_entry("tab-width", "Tab width", "Spaces per tab in diffs", "indent tabs", &["tab_width"]),
            literal_entry(
                "size-units",
                "File sizes",
                "Decimal like Finder (12.4 MB), or binary like git (11.8 MiB)",
                "size units bytes kilobytes megabytes kb mb kib mib finder binary decimal",
                &["size_units"],
            ),
        ],
    ),
];
