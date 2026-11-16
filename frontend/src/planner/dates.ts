function dateParts(iso: string): Date {
  return new Date(`${iso}T12:00:00Z`)
}

export function formatDateShort(iso: string): string {
  return new Intl.DateTimeFormat('ru-RU', {
    day: '2-digit',
    month: '2-digit',
    timeZone: 'UTC',
  }).format(dateParts(iso))
}

export function formatDateNumeric(iso: string): string {
  return new Intl.DateTimeFormat('ru-RU', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    timeZone: 'UTC',
  }).format(dateParts(iso))
}

export function formatWeekRange(startDate: string, endDate: string): string {
  return `${formatDateNumeric(startDate)} — ${formatDateNumeric(endDate)}`
}

export function addDaysIso(iso: string, days: number): string {
  const date = dateParts(iso)
  date.setUTCDate(date.getUTCDate() + days)
  return date.toISOString().slice(0, 10)
}
