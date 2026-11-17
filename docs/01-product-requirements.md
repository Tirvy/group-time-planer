# 01. Product Requirements

## 1. Purpose
Help a small group coordinate recurring weekly activities by collecting each member's preferred times and showing where preferences overlap. The first motivating scenario is a recurring WoW Mythic+ group, but the product must not contain WoW-specific assumptions in its core domain model.

## 2. Core concepts and vocabulary
- **Group:** a shared coordination space.
- **Owner:** the member who created the group or received ownership transfer.
- **Member:** a Telegram user who joined a group.
- **Availability:** a recurring local weekly interval a member is available for.
- **Unselected time:** the member is unavailable. A blank slot is an explicit negative statement, not silence.
- **Setup pending:** member has not saved their calendar yet.
- **Setup completed:** member has saved at least once, even if they selected zero slots.
- **Reference week:** the upcoming Monday through Sunday used to render the current timetable.

## 3. Primary user journey
1. User starts the Telegram bot.
2. User creates a group or opens an invitation.
3. Invitee sees a group preview and explicitly confirms joining.
4. Member opens the Mini App.
5. On first calendar opening, member selects an IANA timezone.
6. Member edits their recurring weekly preference calendar.
7. Member saves preferences.
8. Members can inspect individual calendars and the combined timetable in their own timezone.
9. Group can view a ranked `/besttime` result and export a PNG timetable.

## 4. Calendar model
- Week starts Monday and ends Sunday.
- Each day has 48 half-hour slots; a week has 336 slots.
- Calendar is recurring weekly, not tied to a specific date.
- Store preferences as local weekday/minute intervals and the member's IANA timezone.
- Intervals are half-open: `[start, end)`.
- Valid boundaries are multiples of 30 minutes.
- `start_minute` is in `0..1439`; `end_minute` is in `1..1440`; `start_minute < end_minute`.
- Intervals may not overlap for the same member, group and weekday. Adjacent intervals are allowed; the UI may merge them.
- A selected slot means the member is available. An unselected slot means the member is unavailable.

## 5. Timezone requirements
- Every user has one selected IANA timezone.
- Timezone selection is required when the user first opens a calendar. The Mini App may suggest the device timezone, but the user must confirm it before saving preferences or viewing converted schedules.
- The same timezone is used both to interpret that user's recurring preferences and to display the timetable to that user.
- Store timezone identifiers such as `Asia/Tbilisi`, `Europe/Berlin`, not a fixed UTC offset.
- Do not store recurring preferences as UTC timestamps.
- The timetable defaults to the upcoming Monday–Sunday, calculated in the viewer's timezone.
- Convert each member's recurring local intervals into real instants for that reference week using timezone rules, then display them in the viewer's timezone.
- Use one shared backend timezone-conversion module for API responses, `/besttime`, and PNG rendering.
- Use a maintained IANA timezone library (recommended Rust crate: `chrono-tz`, with `chrono` or an equivalent supported date/time layer).
- MVP should not build a custom timezone engine or future scheduling engine. Document and test a deterministic policy for ambiguous/nonexistent local times caused by daylight-saving transitions. For normal half-hour slots, conversion must be library-based and consistent.
- Show the reference-week date range in the timetable so users know which week's timezone rules were applied.

## 6. Group rules
- Maximum 20 members per group, including owner.
- Creator is automatically the first member and initial owner.
- A Telegram user may belong to multiple groups.
- Preferences are independent per group.
- Each member has a stable color within each group, stored on membership.
- Only owner and member roles are needed in MVP.
- Owner may manage invitations, view schedules, transfer ownership and delete group.
- Members may edit their own preferences, view all group calendars and leave.
- Owner cannot leave while still owner. They must transfer ownership to an existing member first, or delete the group.
- Ownership transfer does not remove the previous owner from membership.

## 7. Preference setup states
- `setup_pending`: no successful save has occurred (`setup_completed_at IS NULL`).
- `setup_completed`: user has saved at least once, even with an empty interval list.
- Empty save is valid and means the member is intentionally unavailable for the whole week.
- Pending members must not be counted as having no preferences.
- Completed members with zero selected slots are included in the denominator for any percentage metric, if percentages are displayed.

## 8. Timetable and ranking
- Every member can view every other member's individual calendar.
- Viewer sees all calendars converted into their own selected timezone.
- Combined timetable shows stable member colors and a legend.
- `/timetable` produces a PNG from SVG using the same reference week and timezone conversion as the interactive view.
- PNG includes group name, reference-week date range, generation date/time, member legend and setup status for pending members.
- `/besttime` ranks half-hour intervals by number of configured members who are available for the corresponding actual time interval in the reference week.
- Adjacent slots with identical ranking counts and the same available members may be merged for presentation.
- Ranking reflects declared weekly availability, not a live presence check or an automatic booking recommendation.
- Pending members are excluded from ranking counts and denominators. Completed members with no selected slots are configured but unavailable all week.

## 9. Group invitations
- Invitation links use Telegram deep linking: `https://t.me/<BOT_USERNAME>?start=join_<TOKEN>`.
- Opening a link only shows a preview; it does not create membership.
- Preview shows group name, owner, current members and count, and whether capacity is available.
- User must press an explicit Russian confirmation button to join.
- Revalidate invitation validity, capacity and membership at confirmation time.
- Joining must be idempotent and transactional.
- Tokens are cryptographically random, opaque, at least 128 bits entropy (use 256 bits by default); store only a hash.
- Invitations support revocation and optional expiration.

## 10. Notifications
Best-effort private notifications to the current owner on:
- New member joins.
- Member leaves.
- A member successfully saves changed preferences.

Notification failure must not roll back the domain action. Telegram bots cannot initiate private chats with users who have never started the bot; explain this limitation and log delivery failures safely. After ownership transfer, notifications go to the new owner.

## 11. Localization
- Default user-facing language is Russian.
- Telegram command identifiers remain English: `/create`, `/groups`, `/invite`, `/calendar`, `/timetable`, `/besttime`, `/members`, `/leave`, `/transfer`, `/delete`, `/help`.
- Bot responses, button labels, notifications, errors, and Mini App interface are Russian.
- Keep user-facing strings centralized for future localization.

## 12. Out of scope
No WoW API integration, character/role management, dungeon readiness checks, automatic event booking, attendance, reminders, recurring event creation, moderator roles, calendar integrations, historical analytics, or automated database backups in MVP.
