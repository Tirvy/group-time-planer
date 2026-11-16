/**
 * Centralized Russian UI strings. Components must not hardcode user-facing text
 * so that future localization can replace this module wholesale.
 */
function participantsRu(count: number): string {
  const mod100 = count % 100
  const word =
    mod100 >= 11 && mod100 <= 14
      ? 'участников'
      : count % 10 === 1
        ? 'участник'
        : count % 10 >= 2 && count % 10 <= 4
          ? 'участника'
          : 'участников'
  return `${count} ${word}`
}

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
    profileNote:
      'Профиль подтверждён Telegram. Часовой пояс сохранён на сервере.',
  },
  planner: {
    dayShort: ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Вс'],
    dayFull: [
      'Понедельник',
      'Вторник',
      'Среда',
      'Четверг',
      'Пятница',
      'Суббота',
      'Воскресенье',
    ],
    groupLabel: 'Группа',
    chooseGroup: 'Выберите группу, чтобы открыть календарь.',
    noGroupsTitle: 'У вас пока нет групп',
    noGroupsBody:
      'Создайте группу в боте командой /create, затем вернитесь в приложение.',
    loadingGroups: 'Загружаем группы…',
    loadingPreferences: 'Загружаем календарь…',
    loadingCalendar: 'Загружаем общий календарь…',
    calendarLoadError: 'Не удалось загрузить календарь группы.',
    tabMine: 'Мой календарь',
    tabCombined: 'Общий вид',
    tabMembers: 'Участники',
    tabBest: 'Лучшее время',
    combinedTitle: 'Общий вид',
    combinedSubtitle: 'Все предпочтения в вашем часовом поясе.',
    referenceWeekLabel: 'Неделя',
    viewerTimezoneLabel: 'Часовой пояс',
    membersPrefer: 'Предпочитают это время',
    nobodyMarked: 'Никто не отметил это время.',
    close: 'Закрыть',
    combinedDirtyNote:
      'Показаны сохранённые предпочтения. Несохранённые изменения не включены.',
    memberCalendarTitle: 'Календарь участника',
    backToMembers: 'К списку участников',
    membersTitle: 'Участники',
    memberScheduleLoading: 'Загружаем календарь участника…',
    memberScheduleError: 'Не удалось загрузить календарь участника.',
    noSegments: 'Нет отмеченного времени.',
    pendingNoCalendar: 'Календарь не настроен.',
    timezoneHint: 'Время показано в вашем часовом поясе.',
    bestTimeTitle: 'Лучшее время',
    bestTimeSubtitle: 'Рейтинг предпочтений за неделю.',
    bestConfigured: 'Настроили календарь',
    bestPending: 'Не настроили',
    bestEmpty: 'Пока нет предпочтений для рейтинга.',
    bestNoConfigured: 'Пока никто не настроил календарь.',
    bestLoadError: 'Не удалось загрузить рейтинг.',
    bestNote:
      'Это предпочтения, а не подтверждённая доступность. Участники без настроенного календаря исключены из рейтинга.',
    downloadPng: 'Скачать PNG',
    downloadingPng: 'Готовим PNG…',
    pngError: 'Не удалось сформировать PNG.',
    preferredBy: (count: number) =>
      count === 1
        ? `предпочитает ${participantsRu(count)}`
        : `предпочитают: ${participantsRu(count)}`,
    retry: 'Повторить',
    owner: 'владелец',
    member: 'участник',
    calendarTitle: 'Ваш недельный календарь',
    calendarSubtitle:
      'Отметьте получасовые интервалы, которые вам предпочтительны.',
    pendingBanner:
      'Календарь ещё не настроен. Отметьте время или сохраните пустой календарь.',
    emptyCompletedBanner:
      'Вы сохранили пустой календарь: предпочтений нет. Это не значит, что время недоступно.',
    configuredBanner: 'Календарь настроен.',
    setupPendingShort: 'календарь не настроен',
    setupCompletedShort: 'календарь настроен',
    unsaved: 'Несохранённые изменения',
    save: 'Сохранить',
    saving: 'Сохраняем…',
    clear: 'Очистить',
    discard: 'Отменить',
    saved: 'Календарь сохранён',
    saveError: 'Не удалось сохранить календарь.',
    loadError: 'Не удалось загрузить календарь.',
    groupsLoadError: 'Не удалось загрузить группы.',
    legendSelected: 'Предпочтительное время',
    legendBlank:
      'Пустая клетка — время не указано, а не недоступно',
    hint:
      'Нажмите на полчаса или проведите по сетке. Повторное нажатие снимает отметку. ' +
      'На телефоне вертикальная прокрутка страницы сохраняется.',
    discardTitle: 'Несохранённые изменения',
    discardBody:
      'Изменения в календаре будут потеряны при переходе к другой группе.',
    stay: 'Остаться',
    leave: 'Перейти без сохранения',
    participants: participantsRu,
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
    NOT_A_MEMBER: 'Вы не являетесь участником этой группы.',
    GROUP_NOT_FOUND: 'Группа не найдена.',
    MEMBER_NOT_FOUND: 'Участник не найден в группе.',
    VALIDATION_ERROR: 'Проверьте введённые данные и попробуйте снова.',
    UNKNOWN: 'Произошла неизвестная ошибка. Попробуйте позже.',
  },
} as const
