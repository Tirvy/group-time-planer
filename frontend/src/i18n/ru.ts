/**
 * Centralized Russian UI strings. Components must not hardcode user-facing text
 * so that future localization can replace this module wholesale.
 */
export const ru = {
  app: {
    title: 'Raid Scheduler',
    subtitle: 'Совместное планирование встреч',
  },
  common: {
    cancel: 'Отмена',
    loading: 'Загрузка…',
    retry: 'Повторить',
    memberFallback: 'Участник',
  },
  auth: {
    checking: 'Проверяем данные Telegram…',
    noTelegramTitle: 'Откройте приложение через Telegram',
    noTelegramBody:
      'Авторизация выполняется только внутри Telegram Mini App. Внешний браузер не может подтвердить вашу личность.',
    errorTitle: 'Не удалось войти',
    expiredHint: 'Закройте и откройте Mini App заново.',
    retry: 'Повторить проверку',
  },
  timezone: {
    title: 'Выберите часовой пояс',
    body:
      'Он будет использоваться и для отметок в календаре, и для показа времени. ' +
      'Часовой пояс можно изменить позже.',
    requiredHint: 'Без выбранного часового пояса календарь недоступен.',
    changeTitle: 'Изменить часовой пояс',
    current: 'Текущий часовой пояс',
    changeWarning:
      'После изменения изменится и трактовка ваших отметок, и время, которое вы видите в календаре.',
    label: 'Часовой пояс',
    searchHint: 'Начните вводить город или регион, например Europe/Berlin.',
    suggested: 'Определён часовой пояс устройства',
    useSuggested: 'Использовать',
    save: 'Сохранить часовой пояс',
    saving: 'Сохраняем…',
    confirmTitle: 'Изменить часовой пояс?',
    confirmBody:
      'Время в календаре и трактовка ваших отметок изменятся. Продолжить?',
    confirm: 'Изменить',
    saved: 'Часовой пояс сохранён',
  },
  home: {
    greeting: (name: string) => `Здравствуйте, ${name}!`,
    timezoneLabel: 'Ваш часовой пояс',
    changeTimezone: 'Изменить часовой пояс',
    calendarTitle: 'Недельный календарь',
    calendarBody:
      'Здесь появится сетка предпочтений: понедельник–воскресенье, получасовые интервалы.',
    calendarNote:
      'Отмеченное время означает предпочтение, а не подтверждённую доступность. ' +
      'Пустая клетка означает, что время не указано.',
    profileNote:
      'Профиль подтверждён Telegram. Часовой пояс сохранён на сервере.',
  },
  errors: {
    NETWORK_ERROR:
      'Не удалось связаться с сервером. Проверьте соединение и попробуйте снова.',
    HTTP_ERROR: 'Сервер вернул неожиданный ответ. Попробуйте позже.',
    AUTH_MISSING:
      'Не удалось подтвердить вход через Telegram. Откройте приложение заново.',
    AUTH_INVALID:
      'Данные Telegram недействительны. Закройте и откройте Mini App заново.',
    AUTH_EXPIRED:
      'Сессия Telegram устарела. Закройте и откройте Mini App заново.',
    AUTH_UNAVAILABLE:
      'Вход через Telegram временно недоступен. Попробуйте позже.',
    TIMEZONE_REQUIRED: 'Сначала выберите часовой пояс.',
    TIMEZONE_INVALID: 'Неизвестный часовой пояс. Выберите значение из списка.',
    VALIDATION_ERROR: 'Проверьте введённые данные и попробуйте снова.',
    UNKNOWN: 'Произошла неизвестная ошибка. Попробуйте позже.',
  },
} as const
