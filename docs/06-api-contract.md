# 06. Mini App API Contract

The exact route names may be refined during implementation, but behavior and authorization requirements are binding. All endpoints return JSON. Use consistent error envelopes and never trust user IDs from request bodies.

## 1. Authentication
Mini App sends Telegram `initData` to the backend, preferably via an `Authorization` header or dedicated exchange endpoint. Backend validates Telegram signature and freshness, resolves/creates the user, and issues a short-lived application session if desired. Every protected route derives user identity from validated server-side auth context.

## 2. Suggested endpoints

### `POST /api/auth/telegram`
Request: `{ "initData": "..." }`
Response: authenticated user summary, timezone state, and session token/cookie if implementing a session exchange.

### `GET /api/me`
Returns current user profile, timezone and timezone-setup status.

### `PUT /api/me/timezone`
Request: `{ "timezone": "Asia/Tbilisi" }`
Validate against IANA timezone database. Persist on user profile. The same timezone governs preference interpretation and timetable display.

### `GET /api/groups`
Returns groups for authenticated user, role, member count, own setup status and group metadata.

### `GET /api/groups/{group_id}`
Requires membership. Returns group summary and members, subject to product visibility rules.

### `GET /api/groups/{group_id}/calendar`
Requires membership and configured user timezone. Returns upcoming Monday–Sunday reference week, viewer timezone, member summaries (including each member's IANA timezone), and converted interval/cell data for all members. Include each member's setup status and stable color. Other members' calendars are read-only.

### `GET /api/groups/{group_id}/members/{user_id}/calendar`
Requires membership in group; target must be current member. Returns that member's preferences converted into viewer timezone and setup state.

### `PUT /api/groups/{group_id}/preferences`
Requires membership and configured timezone. Request contains local recurring intervals, e.g. `{ "intervals": [{"weekday":1,"start_minute":1080,"end_minute":1200}] }`. Backend always saves for authenticated user, never a user ID supplied by client. Validate, normalize and replace atomically. Empty list is valid and marks setup completed.

### `GET /api/groups/{group_id}/best-time`
Requires membership and configured timezone. Returns ranked available intervals for the upcoming reference week, converted to viewer timezone. Include count, configured-member count and the members available in each range; label as declared availability, not a live check.

### `GET /api/groups/{group_id}/timetable.png`
Requires membership and configured timezone. Returns PNG generated from same timetable view model used by interactive calendar. Optionally support an endpoint returning SVG for debugging, but do not expose unnecessary formats by default.

### `POST /api/groups/{group_id}/invitations`
Owner only. Creates invitation with random token, stores hash, returns deep link once. Do not return stored hash.

### `POST /api/invitations/{token}/preview`
Validates token and returns preview only; must not join.

### `POST /api/invitations/{token}/join`
Authenticated Telegram user only. Revalidate invitation, group existence, capacity and existing membership inside transaction. Idempotent for existing member.

### `DELETE /api/groups/{group_id}/invitations/{invitation_id}`
Owner only; revoke invitation.

### `POST /api/groups/{group_id}/transfer-ownership`
Owner only. Request target member ID. Transactionally verify target is existing member and transfer ownership. Former owner remains member.

### `POST /api/groups/{group_id}/leave`
Member only, except current owner. Remove membership and scoped preferences. Owner must transfer or delete.

### `DELETE /api/groups/{group_id}`
Owner only. Explicit confirmation at UI level; backend rechecks owner.

## 3. Response conventions
- Use stable machine-readable error codes, e.g. `AUTH_INVALID`, `TIMEZONE_REQUIRED`, `NOT_A_MEMBER`, `NOT_OWNER`, `GROUP_FULL`, `INVITATION_INVALID`, `INVITATION_EXPIRED`, `OWNER_CANNOT_LEAVE`, `VALIDATION_ERROR`.
- Include Russian-friendly message text or map error codes to centralized Russian frontend strings.
- Telegram IDs should be strings in JSON.
- Times in preference input are local recurring weekday/minute values, not UTC timestamps.
- Converted calendar response should include explicit reference week start/end and viewer timezone.

## 4. Authorization matrix
| Operation | Any authenticated user | Member | Owner |
|---|---:|---:|---:|
| List own groups | Yes | — | — |
| View group / members / calendars | No | Yes | Yes |
| Edit own preferences | No | Yes | Yes |
| Create invitation | No | No | Yes |
| Transfer ownership | No | No | Yes |
| Delete group | No | No | Yes |
| Leave group | No | Yes, unless owner | Owner must transfer first |

## 5. API design cautions
- Never accept `user_id` as authority for whose preferences are being edited.
- Check membership on every group-scoped route.
- Revalidate state inside transactions for race-prone actions.
- Avoid leaking group/member data to non-members through error messages.
- Apply request size limits and sensible rate limits, especially to invitation endpoints and image rendering.
