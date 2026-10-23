# Settings window, profile page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-profile = Profile
    .intro = Every page here is this profile's. A new profile starts as a copy of one you have, without its terminals or agent logins.

settings-profile-name = Name and colour
    .keywords = profile rename avatar letter swatch colour color edge

settings-profile-start = Start new terminals in
    .keywords = folder directory cwd home path open also claim

settings-profile-env = Environment
    .description = Variables this profile's terminals start with
    .keywords = env variable aws path registry export

settings-profile-git = Git identity
    .description = Set in its terminals only; { -brand } never writes to a repository.
    .keywords = author committer name email commit user

settings-profile-logins = Agent logins
    .description = A separate login for each agent.
    .keywords = claude codex account config dir home sign in

## Danger zone

settings-profile-group-danger-zone = Danger zone

settings-profile-delete = Delete profile
    .description = Its terminals, notes, tasks and settings are removed and its windows close.
    .keywords = remove danger

## settings_window/sections/profile.rs (labels, hints and notes drawn by this page's code)

settings-profile-choose-a-folder = Choose a folder
settings-profile-stop-opening-folder = Stop opening this folder in the profile
settings-profile-add-a-folder = Add a folder…
settings-profile-the-default-profile-stays-it-holds = The Default profile stays: it holds the data { -brand } always had.
settings-profile-delete-profile = Delete profile…
settings-profile-cancel = Cancel
settings-profile-delete-button = Delete

## settings_window/sections/profile_form.rs (labels, hints and notes drawn by this page's code)

settings-profile-name-field = Name
settings-profile-the-home-folder = ~ (the home folder)
settings-profile-email = Email
settings-profile-name-value-one-per-line = NAME=value, one per line
settings-profile-choose = Choose

## settings_window/sections/profile_sheet.rs (labels, hints and notes drawn by this page's code)

settings-profile-wormholes-defaults = { -brand }'s defaults
settings-profile-give-the-profile-a-name = Give the profile a name
settings-profile-new-profile = New profile
settings-profile-start-from = Start from
settings-profile-logins-never-copied = Agent logins are never copied: the new profile signs in on its own.
settings-profile-create = Create

## Sentences with a hole in them

# Under "Start new terminals in" while the profile has no folder of its own. { $profile } is the profile's name.
settings-profile-start-caption-none = Where a new terminal starts. A folder you add below opens in { $profile } too.

# The same with one folder. { $folder } is that folder and { $profile } the profile's name.
settings-profile-start-caption-one = Folders under { $folder } open in { $profile } too.

# The same with several. { $folders } are all but the last, separated by commas, and { $last } the last.
settings-profile-start-caption-many = Folders under { $folders } and { $last } open in { $profile } too.

# The question under "Delete profile…". { $profile } is the profile's name.
settings-profile-delete-question = Delete { $profile }? Its terminals, notes, tasks and settings are removed, and its windows close. { -brand } goes to Default.

# Why the environment field is refused. { $line } is the line that is not NAME=value.
settings-profile-env-not-a-pair = “{ $line }” is not NAME=value

# An option of "Start from" in the new profile sheet. { $profile } is the profile that is copied.
settings-profile-start-from-copy = A copy of { $profile }
