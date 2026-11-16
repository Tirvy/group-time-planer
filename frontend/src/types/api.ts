/** Authenticated Mini App user. Telegram IDs are strings. */
export interface UserDto {
  telegram_id: string
  username: string | null
  first_name: string | null
  last_name: string | null
  timezone: string | null
  timezone_set: boolean
}

export interface MeResponse {
  user: UserDto
}

export interface GroupDto {
  id: string
  name: string
  role: 'owner' | 'member'
  color: string
  member_count: number
  setup_completed: boolean
}

export interface GroupsResponse {
  groups: GroupDto[]
}

export interface PreferenceInterval {
  weekday: number
  start_minute: number
  end_minute: number
}

export interface PreferencesResponse {
  group_id: string
  timezone: string
  setup_completed: boolean
  intervals: PreferenceInterval[]
}

export interface CalendarSegment {
  day: number
  start_minute: number
  end_minute: number
  start: string
  end: string
}

export interface CalendarMember {
  user_id: string
  first_name: string | null
  last_name: string | null
  username: string | null
  role: 'owner' | 'member'
  color: string
  setup_state: 'pending' | 'completed'
  segments: CalendarSegment[]
}

export interface CalendarGroup {
  id: string
  name: string
}

export interface ReferenceWeek {
  start_date: string
  end_date: string
  start: string
  end: string
}

export interface GroupCalendarResponse {
  group: CalendarGroup
  viewer_timezone: string
  reference_week: ReferenceWeek
  members: CalendarMember[]
}

export interface MemberCalendarResponse {
  group: CalendarGroup
  viewer_timezone: string
  reference_week: ReferenceWeek
  member: CalendarMember
}
