# Raid Scheduler

Telegram bot + Telegram Mini App for coordinating recurring weekly group availability preferences. The initial example is organizing a regular World of Warcraft Mythic+ group, but the product is game-agnostic.

## Product in one sentence
A group shares a recurring weekly calendar where each member marks times they **prefer**, sees everyone's individual preferences in their own timezone, and gets a ranked view of popular times.

## Documentation map
- [`docs/01-product-requirements.md`](docs/01-product-requirements.md): scope, terminology, product rules.
- [`docs/02-user-stories-and-acceptance.md`](docs/02-user-stories-and-acceptance.md): user stories and testable acceptance criteria.
- [`docs/03-ux-and-telegram-flows.md`](docs/03-ux-and-telegram-flows.md): bot commands, Mini App screens and interaction flows.
- [`docs/04-architecture.md`](docs/04-architecture.md): stack, components, deployment and technical boundaries.
- [`docs/05-data-model.md`](docs/05-data-model.md): SQLite schema and invariants.
- [`docs/06-api-contract.md`](docs/06-api-contract.md): Mini App API contract.
- [`docs/07-security-and-telegram.md`](docs/07-security-and-telegram.md): Telegram authentication, invitations and operational security.
- [`docs/08-implementation-plan.md`](docs/08-implementation-plan.md): phased delivery plan.
- [`docs/09-decisions-and-open-questions.md`](docs/09-decisions-and-open-questions.md): locked decisions and explicitly deferred features.

## Current MVP boundaries
- Rust backend: `teloxide` + `axum` + `SQLx`.
- SQLite database in project-local `data/app.db`, excluded from Git.
- Vue 3 + Vite + Vuetify frontend.
- Telegram Mini App authentication must be validated server-side.
- Weekly Monday–Sunday calendar, 30-minute resolution.
- Per-user IANA timezone, selected on first calendar opening; same timezone controls preference interpretation and display.
- Display the upcoming Monday–Sunday in the viewer's timezone.
- Maximum 20 members per group.
- Click and drag calendar editing.
- Members can view every member's individual calendar.
- Timetable PNG generated from SVG; `/besttime` ranks preferences, not guaranteed availability.
- No automated database backup feature in MVP.

## Important product language
A selected slot means **preferred**. An unselected slot means **no stated preference**, not unavailable. The product must never label the result as confirmed availability.
