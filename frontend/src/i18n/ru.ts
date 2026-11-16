/**
 * Centralized Russian UI strings. Components must not hardcode user-facing text
 * so that future localization can replace this module wholesale.
 */
export const ru = {
  app: {
    title: 'Raid Scheduler',
    subtitle: 'Совместное планирование встреч',
    stageNote:
      'Базовый интерфейс готов. Календарь, группы и приглашения появятся на следующих этапах.',
  },
  status: {
    checking: 'Проверяем соединение с сервером…',
    okTitle: 'Сервис доступен',
    okBody: 'Backend и база данных отвечают нормально.',
    degradedTitle: 'Сервис работает с ограничениями',
    degradedBody: 'База данных сейчас недоступна. Попробуйте повторить проверку позже.',
    errorTitle: 'Сервис недоступен',
    errorBody: 'Не удалось получить состояние сервиса.',
    databaseOk: 'База данных в порядке',
    databaseError: 'База данных недоступна',
    version: 'Версия',
    retry: 'Повторить проверку',
    liveRegion: 'Состояние сервиса',
  },
  common: {
    loading: 'Загрузка…',
  },
  errors: {
    NETWORK_ERROR:
      'Не удалось связаться с сервером. Проверьте соединение и попробуйте снова.',
    HTTP_ERROR: 'Сервер вернул неожиданный ответ. Попробуйте позже.',
    UNKNOWN: 'Произошла неизвестная ошибка. Попробуйте позже.',
  },
} as const
