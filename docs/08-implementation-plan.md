# 07. Security and Telegram Integration

## 1. Mini App authentication
- Frontend must send Telegram Web App `initData` to backend.
- Backend validates the Telegram Web Apps signature using the bot token and Telegram's documented algorithm.
- Validate `auth_date` freshness with a configurable short acceptance window and reject stale data.
- Do not trust `initDataUnsafe` as proof of identity.
- Do not accept a Telegram user ID from frontend request bodies as identity.
- Do not log raw `initData`, bot token, session secrets or invitation tokens.
- Telegram IDs must be handled without JavaScript number precision loss; serialize as strings.
- If issuing app sessions, use secure, HttpOnly cookies where deployment allows, or carefully managed short-lived bearer tokens. Protect state-changing cookie-authenticated requests against CSRF.

## 2. Bot token and configuration
- Store bot token and secrets in environment variables.
- Commit `.env.example` with placeholder values only.
- Ignore `.env`.
- Never print secrets in startup logs or error messages.
- Rotate token if exposed.

## 3. Invitation security
- Generate at least 128 bits of cryptographic randomness; 256 bits recommended.
- Encode token URL-safely.
- Store only a cryptographic hash of token.
- Compare hashes safely and avoid token leakage in logs/analytics.
- Support revocation and expiration.
- Preview is read-only.
- Revalidate token and capacity at join confirmation.
- Bind confirmation to authenticated Telegram user; do not let callback data specify arbitrary user identity.
- Join transaction must be idempotent and enforce max 20 members.

## 4. Authorization
- Backend enforces membership and ownership on every operation.
- Hiding buttons in frontend is not authorization.
- Only authenticated member can view group data.
- Only authenticated user can edit their own preferences.
- Owner-only actions must check current database owner, not stale client role.
- Transfer and deletion must be transactional and revalidated.

## 5. Telegram notification limitations
A bot cannot initiate a private conversation with a user who has never started the bot. Notifications are best effort:
- Domain action succeeds independently of notification delivery.
- Failed delivery is logged safely and may be retried only if a simple retry mechanism is added.
- Do not roll back join/leave/save/transfer/delete because a notification failed.
- Explain to users that they may need to start the bot to receive private notifications.
- After ownership transfer, notify and target the new owner for subsequent events.

## 6. Web and deployment security
- Telegram Mini App must be served over HTTPS.
- Configure trusted origins/CORS narrowly.
- Set request body and upload/render limits.
- Rate-limit invitation preview/join and expensive timetable generation as appropriate.
- Use parameterized SQL / SQLx; never interpolate user input into SQL.
- Validate group names, timezone identifiers, intervals and route IDs.
- Do not expose SQLite file or project data directory through static file serving.

## 7. Data minimization and logs
- Store only data required for product behavior.
- Usernames and names are mutable display metadata, not identity.
- Avoid logging full calendars or personal data unnecessarily.
- Logs should include correlation IDs, action types and safe error categories.
- Do not log bot token, Telegram initData, invitation raw token or session credentials.
