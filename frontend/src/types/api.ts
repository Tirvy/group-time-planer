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
