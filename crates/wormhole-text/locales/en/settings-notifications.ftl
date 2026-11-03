# Settings window, notifications page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-notifications = Notifications
    .intro = What { -brand } tells you about terminals you aren't looking at.

settings-notifications-preview = Live preview
    .keywords = preview banner notification

## Footer

settings-notifications-notify-permission = Banner style and Focus are set by macOS
    .keywords = permission allow system settings send test banner

## Notify me when

settings-notifications-group-notify-me-when = Notify me when

settings-notifications-notify-needs-you = A terminal needs input
    .description = An agent asks a question or for permission
    .keywords = time sensitive focus question permission notifications

settings-notifications-notify-agent-done = An agent finishes
    .description = Its Stop hook, or its output settles. A program that reports it is done or failed counts too
    .keywords = notifications done program report failed error

settings-notifications-notify-command-done = A long command finishes
    .description = Any foreground command, agent or not, that ran longer than
    .keywords = long build threshold seconds notifications

settings-notifications-notify-command-failed = A command fails
    .description = zsh, bash and fish
    .keywords = failed failure error exit code build notifications

settings-notifications-notify-bell = The bell rings
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
    .description = Three or more in 10 s become one
    .keywords = summary bursts

## Dock

settings-notifications-group-dock = Dock

settings-notifications-dock-badge = Badge
    .description = The count on the Dock icon
    .keywords = dock badge count

settings-notifications-dock-bounce = Bounce when a terminal needs input
    .description = Only while { -brand } isn’t the active app; never for a bell
    .keywords = dock bounce

## settings_window/sections/notifications.rs (labels, hints and notes drawn by this page's code)

settings-notifications-banner = Banner
settings-notifications-nothing = Nothing
settings-notifications-needs-you-only = Needs input only
settings-notifications-all = All
settings-notifications-none = None
settings-notifications-off = Off
settings-notifications-needs-you = Needs input
settings-notifications-needs-you-finished = Needs input + finished

## The command threshold in "A command finishes"

# An option of "A command finishes": { $count } is a whole number of minutes, with its unit abbreviated.
settings-notifications-after-minutes = After { $count } min
# The same with seconds. { $count } is a whole number.
settings-notifications-after-seconds = After { $count } s

## settings_window/sections/notifications_preview.rs

