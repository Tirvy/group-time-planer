/**
 * API response types generated from the Rust DTOs by `ts-rs`.
 *
 * The backend is the single source of truth: run
 * `cargo test -p raid-scheduler-backend` after changing a DTO, and CI fails if
 * the checked-in files under `generated/` are stale. Do not edit the generated
 * files by hand.
 *
 * Names are re-exported with the shorter names the app already uses.
 */
export type { BestTimeMemberDto as BestTimeMember } from './generated/BestTimeMemberDto'
export type { BestTimeResponse } from './generated/BestTimeResponse'
export type { CalendarMemberDto as CalendarMember } from './generated/CalendarMemberDto'
export type { CalendarSegmentDto as CalendarSegment } from './generated/CalendarSegmentDto'
export type { GroupCalendarResponse } from './generated/GroupCalendarResponse'
export type { GroupDto } from './generated/GroupDto'
export type { GroupInfoDto as CalendarGroup } from './generated/GroupInfoDto'
export type { GroupsResponse } from './generated/GroupsResponse'
export type { IntervalDto as PreferenceInterval } from './generated/IntervalDto'
export type { MeResponse } from './generated/MeResponse'
export type { MemberCalendarResponse } from './generated/MemberCalendarResponse'
export type { PreferencesResponse } from './generated/PreferencesResponse'
export type { RankedIntervalDto as RankedInterval } from './generated/RankedIntervalDto'
export type { ReferenceWeekDto as ReferenceWeek } from './generated/ReferenceWeekDto'
export type { UserDto } from './generated/UserDto'
