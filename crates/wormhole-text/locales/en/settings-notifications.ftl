# Settings window, notifications page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-notifications = Notifications
    .intro = What { -brand } tells you about terminals you aren't looking at.

## macOS

settings-notifications-group-macos = macOS

settings-notifications-notify-permission = Notifications in System Settings
    .keywords = permission allow system settings send test banner

## Notify me when

settings-notifications-group-notify-me-when = Notify me when

settings-notifications-notify-needs-you = A terminal needs you
    .description = An agent asks a question or for permission
    .keywords = time sensitive focus question permission notifications

settings-notifications-notify-agent-done = An agent finishes
    .description = Its Stop hook, or its output settles. A program that reports it is done or failed counts too
    .keywords = notifications done program report failed error

settings-notifications-notify-command-done = A command finishes
    .description = Any foreground command, agent or not, that ran longer than
    .keywords = long build threshold seconds notifications

settings-notifications-notify-command-failed = A command fails
    .description = Non-zero exit code. Needs shell integration (on for zsh, bash and fish)
    .keywords = failed failure error exit code build notifications

settings-notifications-notify-bell = The terminal rings the bell
    .description = Terminals without an agent; at most once a minute each
    .keywords = bell notifications

## Delivery

settings-notifications-group-delivery = Delivery

settings-notifications-notify-only-unseen = Only for terminals I’m not looking at
    .description = Off: the terminal in front notifies too
    .keywords = unseen front

settings-notifications-notify-in-front = While { -brand } is in front
    .description = A system banner, or nothing
    .keywords = banner nothing front

settings-notifications-notify-sound = Sound
    .keywords = sound alert

## Group bursts

settings-notifications-notify-group-bursts = Group bursts
    .description = Three or more within 10 s become one summary
    .keywords = summary bursts

## Dock

settings-notifications-group-dock = Dock

settings-notifications-dock-badge = Badge
    .description = The count on the Dock icon
    .keywords = dock badge count

settings-notifications-dock-bounce = Bounce once when a terminal needs you
    .description = Only while { -brand } isn’t the active app; never for a bell
    .keywords = dock bounce
