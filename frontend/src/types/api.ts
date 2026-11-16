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
