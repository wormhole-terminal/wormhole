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
