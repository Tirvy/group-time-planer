# 02. User Stories and Acceptance Criteria

## Group creation
**As a user**, I can create a group with a name so that I can invite friends.

Acceptance:
- `/create` starts a guided flow.
- Name is trimmed, non-empty and within a documented length limit.
- Creator is inserted as owner and first member atomically.
- Group starts with zero invitations and creator setup pending until they save preferences.

## Group list and selection
**As a user**, I can see groups I belong to and choose the active group.

Acceptance:
- `/groups` lists only current memberships.
- If the user has no groups, offer create/join actions.
- If exactly one group is available, calendar actions may open it directly.
- If multiple groups exist, ask the user to choose; do not guess based on stale context.

## Invitation preview and joining
Acceptance:
- Opening a valid deep link displays a preview and does not join.
- Preview displays group name, owner, member count and member names, plus capacity status.
- Confirmation callback is bound to the Telegram user who opened the preview.
- Confirmation revalidates token hash, expiry, revocation, group existence, capacity and membership.
- Repeated confirmation does not create duplicate membership.
- At capacity (20), reject join with a clear Russian message.
- If already a member, show already-member response and calendar shortcut.
- Successful join creates a pending calendar setup state and notifies current owner best-effort.

## Timezone setup
Acceptance:
- First calendar opening requires timezone selection.
- UI may preselect device/browser timezone as a suggestion, but user explicitly confirms.
- Persist IANA timezone on user profile.
- Invalid timezone identifiers are rejected.
- User may change timezone later after a clear explanation that displayed times and interpretation of their recurring local preferences will change.
- No calendar or ranking view may silently fall back to server timezone.

## Calendar editing
Acceptance:
- Calendar shows Monday–Sunday and 48 half-hour slots per day.
- User can toggle an individual slot.
- User can select/deselect a continuous range using pointer drag.
- Mobile scrolling remains usable; drag interaction must not make the calendar impossible to scroll.
- User can clear a range.
- Save replaces that user's intervals for the active group atomically.
- Empty save is valid and marks setup complete.
- Unsaved changes are indicated; navigating away prompts to discard or stay.
- Only the authenticated user can edit their own preferences.

## Calendar visibility
Acceptance:
- Every group member can view every member's individual calendar.
- Individual calendar view includes member name and setup status.
- Pending member is labelled as not configured, not as having zero preferences.
- All calendars are displayed in the viewer's timezone.
- A member's own calendar is visually distinguished.

## Timetable
Acceptance:
- Combined view shows member colors and legend.
- Each view includes the upcoming Monday–Sunday reference-week dates in viewer timezone.
- Timezone conversion is performed by backend shared logic.
- PNG matches the interactive timetable for the same group, viewer timezone and reference week.
- PNG includes group name, date range, generation timestamp, legend and pending setup indicators.

## Best time
Acceptance:
- Rank slots by count of completed members who prefer the corresponding actual interval in the reference week.
- Pending members are excluded.
- Completed members with zero preferences count as configured but add no votes.
- Merge adjacent equal-score intervals for readability.
- Label output as preferences, not confirmed availability.

## Membership and ownership
Acceptance:
- `/members` lists owner first, then other members, including setup status.
- Provide a clickable Telegram link only when a valid username or supported safe link is available; never construct links from database IDs.
- Regular member can leave after confirmation.
- Leaving removes only membership-scoped preferences for that group.
- Owner cannot leave while owner.
- Owner can transfer ownership to an existing member after confirmation.
- Former owner remains a member after transfer.
- New owner receives subsequent owner notifications.
- Owner can delete group after explicit confirmation; group-specific dependent data is deleted by cascade.

## Reliability and security
Acceptance:
- Mini App requests are authenticated using validated Telegram `initData`.
- Client-supplied Telegram user IDs are never trusted.
- All group operations enforce membership and role checks on the backend.
- Domain writes use transactions and database constraints for invariants.
- Telegram notification failures do not undo successful domain writes.
- Secrets and raw authentication data are not logged.
