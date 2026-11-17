# 03. UX and Telegram Flows

## 1. Main bot menu
All user-facing text is Russian. Suggested main actions:
- «Мои группы»
- «Создать группу»
- «Открыть календарь»
- «Расписание картинкой» — отправляет PNG-расписание активной группы
- «Помощь»

Use Telegram commands as shortcuts, not as the only discoverable navigation.

## 2. Commands
| Command | Behavior |
|---|---|
| `/create` | Guided group creation |
| `/groups` | List/select groups |
| `/invite` | Create or show invitation for selected group |
| `/calendar` | Open Mini App calendar for selected group |
| `/timetable` | Generate and send PNG timetable |
| `/besttime` | Show ranked preference intervals |
| `/members` | Show group members and setup status |
| `/leave` | Leave group, with confirmation; owner must transfer/delete first |
| `/transfer` | Transfer ownership to an existing member |
| `/delete` | Delete group, owner only, with confirmation |
| `/help` | Explain the product and commands |

If a command requires a group and the user belongs to multiple groups, show a group picker. If there is exactly one group, it may be selected automatically. If there are no groups, show create/join guidance.

## 3. Invitation preview
Deep link: `https://t.me/<BOT_USERNAME>?start=join_<TOKEN>`.
1. Parse start payload and locate token hash.
2. Validate that invitation exists, is active, not expired, group exists and group has capacity.
3. Show group name, owner, member count, member names and «Вступить в группу» button.
4. Do not create membership at preview time.
5. On confirmation, revalidate everything inside the membership transaction.
6. On success, show «Вы вступили в группу» and a «Настроить календарь» action.
7. Notify current owner best-effort.

## 4. First calendar opening / timezone selection
1. Authenticate Mini App request using Telegram `initData`.
2. If the user's timezone is absent, block calendar view and show timezone selection.
3. Suggest browser-detected IANA timezone where available, but require explicit confirmation.
4. Save timezone to user profile.
5. Continue to calendar, showing upcoming Monday–Sunday in that timezone.

Timezone selector should support search by city/region and show the actual IANA identifier or a clear city/region label. Do not store a fixed UTC offset.

## 5. Calendar editor
- Week columns/days: Monday through Sunday.
- Vertical time axis: 00:00 through 24:00.
- Cell size: 30 minutes.
- Selected cells use the member's stable group color (or a strong selected treatment in own editor).
- Tap/click toggles one cell.
- Pointer drag paints or erases a continuous interval based on the initial cell state.
- Provide clear/reset action.
- Save and discard actions are explicit.
- Warn on unsaved navigation.
- Mobile: preserve vertical scrolling; do not require drag for basic editing.

## 6. Viewing individual calendars
From the member list or timetable, select a member to open their individual calendar. All members can view all individual calendars. Display member name, stable color, setup status, and reference week. Read-only for other members.

## 7. Combined timetable
Show all members' availability using their stable colors. Include a legend, pending setup state and reference-week date range. Display times in the viewer's selected timezone. The view must make it clear that a blank cell means "unavailable", not "unknown".

## 8. Best-time output
Display ranked intervals with count, for example:
- «Понедельник, 19:00–20:30 · доступны 5 участников».
Use wording equivalent to “available”, never imply a live presence check. Explain that unconfigured members are excluded.

## 9. Ownership transfer
1. Owner invokes `/transfer`.
2. Show existing members except current owner.
3. Owner selects target and confirms.
4. Backend revalidates owner and target membership in a transaction.
5. Update owner; old owner remains member.
6. Notify new owner best-effort.

## 10. Leave and delete
- Regular member: `/leave`, show group name and confirmation; on confirm remove membership and scoped preferences; notify owner.
- Owner: cannot leave while owner. Offer transfer ownership or delete group.
- Delete: owner-only confirmation that clearly states all group data will be deleted. Capture member Telegram IDs before cascade deletion, then best-effort notify them.

## 11. Visual design
Use Vuetify components and a consistent design system. Avoid generic unstyled scaffolding. Prioritize a polished compact mobile-first Mini App. Use accessible contrast, clear selected/unselected states, touch targets, loading/error/empty states and Russian copy. Do not overload the weekly grid with decorative elements.
