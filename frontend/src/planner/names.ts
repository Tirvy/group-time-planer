import { ru } from '../i18n/ru'
import type { CalendarMember } from '../types/api'

type NameParts = Pick<CalendarMember, 'first_name' | 'last_name' | 'username'>

export function memberDisplayName(member: NameParts): string {
  const first = member.first_name?.trim() ?? ''
  const last = member.last_name?.trim() ?? ''

  if (first && last) return `${first} ${last}`
  if (first) return first
  if (member.username) return `@${member.username}`
  return ru.common.memberFallback
}

export function memberRoleLabel(member: Pick<CalendarMember, 'role'>): string {
  return member.role === 'owner' ? ru.planner.owner : ru.planner.member
}

export function memberSetupLabel(
  member: Pick<CalendarMember, 'setup_state'>,
): string {
  return member.setup_state === 'pending'
    ? ru.planner.setupPendingShort
    : ru.planner.setupCompletedShort
}
