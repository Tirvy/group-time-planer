const FALLBACK_TIMEZONES: string[] = [
  'UTC',
  'Europe/London',
  'Europe/Dublin',
  'Europe/Lisbon',
  'Europe/Madrid',
  'Europe/Paris',
  'Europe/Brussels',
  'Europe/Amsterdam',
  'Europe/Berlin',
  'Europe/Zurich',
  'Europe/Vienna',
  'Europe/Prague',
  'Europe/Warsaw',
  'Europe/Budapest',
  'Europe/Rome',
  'Europe/Athens',
  'Europe/Bucharest',
  'Europe/Kyiv',
  'Europe/Istanbul',
  'Europe/Moscow',
  'Asia/Tbilisi',
  'Asia/Yerevan',
  'Asia/Baku',
  'Asia/Dubai',
  'Asia/Almaty',
  'Asia/Tashkent',
  'Asia/Karachi',
  'Asia/Kolkata',
  'Asia/Dhaka',
  'Asia/Bangkok',
  'Asia/Jakarta',
  'Asia/Shanghai',
  'Asia/Hong_Kong',
  'Asia/Singapore',
  'Asia/Tokyo',
  'Asia/Seoul',
  'Australia/Perth',
  'Australia/Adelaide',
  'Australia/Brisbane',
  'Australia/Sydney',
  'Pacific/Auckland',
  'America/Sao_Paulo',
  'America/Argentina/Buenos_Aires',
  'America/Santiago',
  'America/Bogota',
  'America/Lima',
  'America/New_York',
  'America/Toronto',
  'America/Chicago',
  'America/Denver',
  'America/Phoenix',
  'America/Los_Angeles',
  'America/Vancouver',
  'America/Anchorage',
  'Pacific/Honolulu',
]

/** Returns a sorted list of IANA identifiers, preferring the browser database. */
export function listTimezones(): string[] {
  const intl = Intl as unknown as {
    supportedValuesOf?: (key: string) => string[]
  }
  const supported = intl.supportedValuesOf?.('timeZone') ?? FALLBACK_TIMEZONES
  return [...new Set([...supported, 'UTC'])].sort()
}

/** Suggests the device timezone without saving it. */
export function deviceTimezone(): string | null {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || null
  } catch {
    return null
  }
}

/** Human-readable current UTC offset for display next to an IANA identifier. */
export function formatOffset(zone: string): string {
  try {
    const parts = new Intl.DateTimeFormat('en-US', {
      timeZone: zone,
      timeZoneName: 'longOffset',
    }).formatToParts(new Date())
    const value = parts.find((part) => part.type === 'timeZoneName')?.value ?? ''
    return value.replace('GMT', 'UTC')
  } catch {
    return ''
  }
}
