# Название языка на нём самом.
language-name = Русский

## Стартовый экран

gate-preparing = Подготовка
gate-checking = Проверка обновлений
gate-fetching = Загрузка каталога
gate-pictures = Загрузка изображений
gate-installing = Установка
gate-offline-title = Нет соединения
gate-offline-hint = Каталог не получен
gate-offline-retry = Повторить
gate-outdated-title = Требуется обновление
gate-outdated-hint = Данные новее этой сборки
gate-stale-title = Данные старее сборки
gate-stale-hint = Опубликованный каталог этой сборкой не читается

## Заголовок окна

titlebar-minimize = Свернуть
titlebar-maximize = Развернуть
titlebar-close = Закрыть

## Всплывающие сообщения

toast-dismiss = Скрыть
toast-command-failed = Команда не выполнена

## Таймеры

timers-title = Сводка
timers-fissures = Разломы
timers-waiting = Данные не получены
timers-stale = обновлено { $minutes } мин назад

## Циклы миров

cycle-name = { $cycle ->
    [cetus] Цетус
    [earth] Земля
    [vallis] Долина Сфер
    [cambion] Камбионский Дрейф
    [duviri] Дувири
   *[other] { $cycle }
  }
cycle-phase = { $phase ->
    [day] день
    [night] ночь
    [warm] тепло
    [cold] холод
    [fass] Фэз
    [vome] Воум
    [joy] радость
    [anger] ярость
    [envy] зависть
    [sorrow] печаль
    [fear] страх
   *[other] { $phase }
  }
# Смена фазы цикла.

## Разломы

era-name = { $era ->
    [lith] Лит
    [meso] Мезо
    [neo] Нео
    [axi] Акси
    [requiem] Реквием
    [omnia] Омния
    [vanguard] Авангард
    [void] Пустота
   *[other] { $era }
  }
# Короткие пометки на разломе.
fissure-hard = СП

## Сбросы

reset-name = { $reset ->
    [daily] Ежедневный сброс
    [sortie] Вылазка
    [weekly] Недельный сброс
   *[other] { $reset }
  }

## Торговцы

trader-name = { $vendor ->
    [baro] Баро Ки'Тир
    [varzia] Варзия
   *[other] { $vendor }
  }
trader-here = на месте
trader-road = в пути
trader-until-arrival = приедет через
trader-empty = Нет предложений

## Полка торговца

shelf-group-label = Вид
shelf-search = Поиск по названию
shelf-nothing = Ничего не найдено

## Справка о предмете

card-inside = Содержимое
card-empty = Нет данных
card-close = Закрыть
# $percent — доля от ста.
card-chance = { $percent } %
card-less = Свернуть
card-more = Ещё { $count }
# Вид источника; ключ — как в каталоге.
card-source-kind = { $kind ->
    [relic] Реликвии
    [node] Узлы
    [bounty] Задания
    [key] Ключи
    [transient] Временные
    [sortie] Вылазка
    [enemy] Враги
    [event] События
    [vendor] Продавцы
    [lab] Лаборатории
   *[other] { $kind }
  }
card-rotation = ротация { $rotation }
card-rank = ранг { $rank }

## Инвентарь

inventory-title = Инвентарь
inventory-waiting = Нет данных инвентаря

## Строка инструментов

sort-direction = { $direction ->
    [up] По возрастанию
   *[down] По убыванию
  }

## Фильтры

filter-shown = Показано { $shown } из { $count }
filter-class-any = Все классы

## Стена

price-age-fresh = только что
price-age-minutes = { $count } мин
price-age-hours = { $count } ч
price-age-days = { $count } д

## Торговая панель

deal-orders-asking = Загрузка ордеров
deal-status-ingame = В игре
deal-status-online = В сети
deal-status-offline = Не в сети
deal-orders-none = Ордеров нет
orders-rank-pick = { $rank ->
    [max] Ранг { $max }
   *[zero] Ранг 0
  }
deal-price = Цена

## График цены

chart-closed = Сделки

## Словари

rarity-name = { $rarity ->
    [common] обычная
    [uncommon] необычная
    [rare] редкая
   *[other] { $rarity }
  }
# Валюта; ключ — как в ядре.
currency-short = { $currency ->
    [ducats] дук
    [credits] кр
    [aya] айя
    [regalAya] кор. айя
   *[other] { $currency }
  }
# Валюта продавца; ключ — имя из каталога строчными словами через дефис.
vendor-currency = { $currency ->
    [animo-nav-beacon] Навигационный Маяк Анимо
    [aya] Айя
    [beating-heartstrings] Сердечные Струны
    [corrupted-holokey] Испорченный Голоключ
    [cred] Доверие
    [credits] Кредиты
    [dominus-aureus] Ауреус Доминуса
    [ducats] Дукаты
    [enigma-gyrum] Энигма Гирум
    [fate-pearl] Жемчужина судьбы
    [fergolyte] Ферголит
    [grandmother-token] Символ: Бабушка
    [kullervos-bane] Проклятие Куллерво
    [lua-thrax-plasm] Траксовая Плазма Луа
    [maphica] Мафика
    [mother-token] Символ: Мать
    [nakak-pearl] Жемчужины Накак
    [pathos-clamp] Струбцина пафоса
    [phasic-cells] Фазовые Ячейки
    [pix-chip] Чипсель
    [platinum] Платина
    [riven-sliver] Осколок разлома
    [scarlet-credits] Алые Кредиты
    [scuttler-husk] Панцирь Скаттлера
    [standing] Репутация
    [steel-essence] Эссенция стали
    [stock-kahl] Запас
    [vessel-capillaries] Капилляры сосуда
    [vestigial-motes] Рудиментарные частицы
    [vitus-essence] Эссенция Витус
    [voidplume-pinion] Перо Бездны: Маховое
    [volatile-motes] Нестабильные частицы
   *[other] { $name }
  }
# Что вариант говорит о копии.
detail-rank = ранг { $rank }
detail-stars = { $stars ->
    [one] { $stars } звезда
    [few] { $stars } звезды
   *[other] { $stars } звёзд
  }
detail-veiled = нераскрытый

## Настройки

settings-title = Настройки

settings-general = Общие
settings-general-note = Язык и уведомления
settings-appearance = Внешний вид
settings-appearance-note = Цвет выделения
settings-updates = Обновления
settings-updates-note = Версия приложения и обновления
settings-build = Сборка
settings-channel = Канал
release-channel = { $channel ->
    [stable] stable
    [preview] preview
   *[dev] dev
  }
settings-channel-hint = preview получает изменения раньше stable
settings-channel-local = Локальная сборка не обновляется
settings-log = Журнал работы
settings-log-hint = Файл median.log — приложите его к сообщению об ошибке
settings-log-open = Открыть папку
settings-data = Данные игры
settings-data-date = от { $date }
settings-data-game = Сборка игры { $build }
settings-changes = Что изменилось в этой версии
settings-changes-local = У локальной сборки нет списка изменений
settings-changes-none = В этой версии нет записанных изменений
# $revision — «r8217».
update-checking = Проверка новой версии
update-skip = Пропустить
update-unchecked = Новая версия не проверялась
update-latest = Установлена последняя версия
update-waiting = Ждём { $channel } новее { $revision }
update-check = Проверить
update-check-failed = Проверка не удалась: { $reason }
update-found = Доступна { $revision }
update-later = Позже
update-install = Обновить
update-downloading = Загрузка { $revision }
update-downloading-share = Загрузка { $revision }: { $percent }%
update-installing = Установка
update-installing-hint = Приложение закроется и запустится заново
update-failed = Не удалось обновиться
update-install-failed = Не удалось обновиться: { $reason }
update-continue = Продолжить
settings-interface-language = Язык интерфейса
settings-language-hint = Интерфейс и названия предметов из каталога
settings-whisper-sound = Звук при личном сообщении
settings-whisper-sound-hint = Когда вам пишут в игре
settings-notifications = Уведомления

settings-language = Язык

settings-accent = Акцент
# Имена акцентов; ключ — как акцент назван в ядре.
accent-name = { $accent ->
    [ice] Лёд
    [indigo] Индиго
    [amethyst] Аметист
    [orchid] Орхидея
    [steel] Сталь
    [terracotta] Терракота
   *[other] { $accent }
  }

settings-build-version = Версия

account-email = Почта
account-password = Пароль
account-sign-in = Войти
account-sign-out = Выйти
account-expired = Сессия истекла, войдите заново
status-auto = Авто
status-title = Статус на рынке
status-online = Онлайн
status-ingame = В игре
status-invisible = Невидимый
status-out = Не в сети

orders-title = Ордера
orders-selling = Продажа
orders-buying = Покупка
orders-edit-save = Сохранить
orders-none = Ордеров нет
orders-loading = Обновляем ордера

policy-capital = Капитал
policy-most-per-order = Штук в заявке
policy-a-trade = Минимум со сделки

figures-group = { $group ->
    [riven] Ривены
    [relic] Реликвии
    [arcane] Мистификаторы
    [mod] Моды
    [set] Комплекты
    [prime] Прайм
   *[other] Прочее
  }
whisper-writes = Пишет { $player }
alert-volume = Громкость
alert-listen = Прослушать

deal-record-buy = Купил
deal-record-sell = Продал
deal-record-price = Цена за штуку
deal-record-quantity = Количество
deal-record-save = Записать
hand-to-agent = Передать агенту
hand-to-agent-title = Передать агенту
hand-to-agent-note = Агент выставит копии на продажу и не опустит цену ниже цены покупки.
hand-to-agent-price = Цена покупки за штуку
hand-to-agent-quantity = Количество
hand-to-agent-save = Передать

# The layer drawn over the game.
overlay-whole = сет
overlay-built = собрано
overlay-mastered = освоено
overlay-updating = обновление
overlay-per-day = { $count } в день
riven-cycle-reading = Распознавание
riven-cycle-unread = { $why ->
    [noReader] Модель распознавания не загрузилась
    [notHeld] Ривен не найден в инвентаре
    [unsupported] Оружие не поддерживается
   *[stats] Статы не распознаны
  }
riven-cycle-pricing = Расчёт цен
riven-cycle-appraising = Оценка

## Реликвии

relics-title = Реликвии
relics-search = Реликвия или награда
relics-era = Эра
relics-any-era = Все
relics-squad = Отряд
relics-sort = Порядок
relics-by-platinum = По платине
relics-by-ducats = По дукатам
relics-by-upgrade = По приросту
relics-by-count = По количеству
relics-none = Ничего не нашлось
relic-vaulted = хранилище
relic-refinement = { $refinement ->
    [intact] Нетронутая
    [exceptional] Необычная
    [flawless] Бесподобная
    [radiant] Сияющая
   *[other] { $refinement }
  }

## Мастерство и каталог

mastery-title = Мастерство
catalogue-title = Каталог

span-days = { $days } д
span-days-hours = { $days } д { $hours } ч
span-hours-minutes = { $hours } ч { $minutes } мин
span-minutes = { $minutes } мин

mastery-rank-label = ранг
mastery-progress = { $total } / { $next }
mastery-progress-floor = ≥ { $total } / { $next }
mastery-remaining = ещё { $remaining }
mastery-test-open = тест открыт
mastery-test-shut = тест через { $left }
mastery-undercounted = меньше порога ранга
mastery-uncounted = { $source ->
    [items] без части предметов
    [nodes] без узлов
   *[other] { $source }
  }
mastery-source = { $source ->
    [items] Предметы
    [nodes] Звёздная карта
    [steelPath] Стальной путь
    [junctions] Врата
    [steelJunctions] Врата стального пути
    [railjack] Рейлджек
    [drifter] Дрифтер
   *[other] { $source }
  }
mastery-share = { $done } / { $of }
mastery-search = Предмет
mastery-shelf = { $shelf ->
    [warframe] Варфреймы
    [primary] Основное
    [secondary] Вторичное
    [melee] Ближний бой
    [modular] Модульное
    [archwing] Арчвинг
    [companion] Компаньоны
    [companion-weapon] Оружие компаньонов
   *[other] { $shelf }
  }
mastery-gain = +{ $points }
mastery-gain-rank = +{ $points }, ранг { $rank }
mastery-view-items = Предметы
mastery-in-hand = на руках { $points }
rank-gate = Ранг мастерства { $rank }
mastery-none = Ничего не нашлось
mastery-sources = Источники опыта

roadmap-clear = Очистить
roadmap-clear-confirm = Точно?
chain-ready = Можно собрать
chain-everywhere = Везде
roadmap-sum-gain = Опыт
roadmap-gain = +{ $gain }
roadmap-gain-rank = хватит на ранг { $rank }
roadmap-sum-credits = Кредиты
roadmap-sum-time = В печи

catalogue-search = Поиск по каталогу
catalogue-class = Класс
catalogue-ready = можно собрать
catalogue-none = Ничего не нашлось

nav-trading = Торговля
nav-collapse = Свернуть панель
nav-expand = Развернуть панель
inventory-search = Поиск по инвентарю
inventory-nothing-found = Ничего не найдено
own-selling = Продаю
own-buying = Покупаю
nav-world = Мир
timers-cycles = Циклы
fissure-plain = Обычные
fissure-hard-long = Стальной путь
fissure-all = Все
fissure-railjack = Рейлджек
trader-rotation = Смена набора
alarms-title = Уведомления
alarms-none = Пока ничего
alarms-clear = Очистить
alarms-dismiss = Убрать
alarms-watches = Подписки
alarms-no-watches = Нет подписок
alarms-drop = Отписаться
watch-phase = Уведомить о смене фазы
watch-arrival = Уведомить о приезде
fissure-watch-plain = обычный
fissure-watch-hard = Стальной путь
fissure-watch-storm = Рейлджек
alarm-arrival = приезд
alarm-rotation = смена набора
mission-type = { $mission ->
    [MT_ALCHEMY] Алхимия
    [MT_ARMAGEDDON] Армагеддон бездны
    [MT_ARTIFACT] Сбой
    [MT_ASSAULT] Штурм
    [MT_CAPTURE] Захват
    [MT_CORRUPTION] Потоп бездны
    [MT_DEFENSE] Оборона
    [MT_EXCAVATE] Раскопки
    [MT_EXTERMINATION] Зачистка
    [MT_HIVE] Улей
    [MT_INTEL] Шпионаж
    [MT_MOBILE_DEFENSE] Мобильная оборона
    [MT_RESCUE] Спасение
    [MT_SABOTAGE] Саботаж
    [MT_SURVIVAL] Выживание
    [MT_TERRITORY] Перехват
    [MT_VOID_CASCADE] Каскад бездны
   *[other] { $mission }
  }
watch-any-era = Любая эра
watch-any-mission = Любая миссия
watch-take = Подписаться
timers-resets = Сбросы
timers-rotations = Ротации
shelf-tab-warframes = Варфреймы
shelf-tab-weapons = Оружие
shelf-tab-mods = Моды
shelf-tab-relics = Реликвии
shelf-tab-cosmetics = Внешний вид
shelf-tab-other = Прочее
shelf-tab-bundles = Наборы
shelf-tab-regal = Королевская айя
item-held = есть ×{ $count }
item-mastered = освоено
item-favourite = Избранное
shelf-tab-primes = Праймы
shelf-per-run = пл за забег
shelf-no-drops = нет в реликвиях
shelf-sort = Порядок
shelf-sort-payback = Окупаемость
item-trades = { $count } { $count ->
    [one] сделка
    [few] сделки
   *[many] сделок
  }
item-whole = Комплект
item-sources = Источник
item-daily = Сделок в день
item-range = Разброс
item-period = Период
item-days = { $days } д
item-runs = ≈ { $runs }
item-copy = Скопировать сообщение
item-refinement = Очистка
item-untraded = Не торгуется
item-no-trades = Сделок не было
shelf-missing-only = Нет в наличии
shelf-all = Все
order-status = Статус
order-player = Игрок
order-reputation = Репутация
order-quantity = Кол-во
order-copied = Скопировано
fissure-reset = Сбросить
shelf-unknown = Нет в каталоге
shelf-sort-top = Дорогая награда
shelf-sort-held = Количество
orders-failed = Рынок не ответил
orders-retry = Повторить
inventory-tab = { $tab ->
    [parts] Запчасти
    [mods] Моды
    [arcanes] Мистификаторы
    [sets] Сеты
   *[other] Прочее
  }
inventory-order = { $order ->
    [platinum] Платина
    [ducats] Дукаты
    [count] Количество
    [rank] Уровень
   *[other] Имя
  }
inventory-collections = Все
inventory-uncollected = Прочее
inventory-rank = { $rank }/{ $max }
inventory-to-max = { $count ->
    [0] хватает
   *[other] ещё { $count }
}
inventory-equipped = установлен
inventory-free = своб. { $count }
inventory-fitted = на { $count }
inventory-more = и ещё { $count }
item-crafted = собрано
facet-title = Фильтры
facet-built = Освоено
facet-unbuilt = Не освоено
facet-many = Больше одного
facet-single = Один
facet-vaulted = В хранилище
facet-unvaulted = Не в хранилище
facet-listed = Ордер размещён
facet-unlisted = Без ордера
facet-prime = Прайм
facet-plain = Обычные
facet-favourite = В избранном
facet-unfavoured = Не в избранном
facet-equipped = Установлен
facet-free = Свободен
facet-upgraded = Улучшен
facet-unranked = Не улучшен
facet-spare = Излишек
facet-unspared = Без излишка
facet-tradable = Торгуется
facet-untradable = Не торгуется
facet-held = В инвентаре
facet-unheld = Нет в инвентаре
facet-mastered = Освоенные
facet-unmastered = Неосвоенные
facet-planned = В плане
facet-unplanned = Не в плане
facet-reachable = Хватает ранга
facet-unreachable = Не хватает ранга
facet-ready = Можно собрать
facet-unready = Нельзя собрать
facet-relics = Из своих реликвий
facet-no-relics = Не из своих реликвий
facet-opened = Открыто рангом
facet-closed = Закрыто рангом
facet-affordable = Хватает репутации
facet-unaffordable = Не хватает репутации
facet-beaten = Перебит
facet-best = Лучший
facet-visible = Видимые
facet-hidden = Скрытые
facet-agent = От агента
facet-mine = Свои
sell-quantity = Количество
sell-price = Цена
sell-place = Выставить
sell-placed = Выставлено
sell-signed-out = Нет входа на warframe.market
order-lot = по { $count } шт · { $sum }
sell-rank = Уровень
sell-per-trade = За сделку
sell-held = есть { $count }
troubles-title = Неполадки
trouble-gep-outdated = Коллектор другой версии
trouble-dbwin-closed = Поток игры не читается
trouble-dbwin-shared = Поток игры читает другое приложение
trouble-task-restarting = { $name }: перезапуск
trouble-task-stopped = { $name }: остановлено
trouble-wfm-silent = Рынок не отвечает
trouble-gep-refused = Overwolf не отдаёт данные игры
collector-folder-open = Открыть папку коллектора
trouble-collector-missing = Коллектор не подключён
trouble-ocr-failed = Распознавание наград не работает
trouble-since = с { $time }
trade-positions = Позиции
trade-by-agent = Выставил агент
trade-price = Цена
trade-reference = Лучшая
trade-median = Медиана
trade-quantity = Кол-во
trade-held = Есть
trade-most = Предел
trade-hide = Скрыть
trade-show = Показать
trade-more = Ещё
trade-withdraw = Снять
trade-basis = Куплено
trade-ingame = Рынок
trade-asking = Наш ордер
trade-order = { $order ->
    [gap] Отставание
    [worth] Сумма
    [price] Цена
    [quantity] Количество
   *[other] Имя
  }
trade-refresh = Обновить с рынка
trade-show-all = Показать все
trade-hide-all = Скрыть все
trade-withdraw-all = Снять все: { $count }
trade-withdraw-all-confirm = Подтвердить снятие: { $count }
trade-no-positions = Позиций нет
trade-period = { $days ->
    [0] Всё
   *[other] { $days } дн
  }
agent-name = Агент
agent-bids = Ставки
agent-no-bids = Ставок нет
agent-blocker = { $blocker ->
    [signed-out] Нет входа на WFM
    [purse] Кошелёк не прочитан
    [budget] Бюджет 0
   *[exchanges] Обменов нет
  }
haul-title = Сессия
haul-continue = Продолжить
haul-close = Закрыть сессию
syndicates-title = Синдикаты
syndicate-rank = Ранг { $rank }
syndicate-standing = { $tag ->
    [KahlSyndicate] Запас
   *[other] Репутация
  }
syndicate-today = Сегодня ещё
syndicates-unread = Нет данных о репутации
syndicates-all = Все синдикаты
syndicate-offers = { $count } на продажу
syndicate-unmet = Не встречен
syndicate-needs-rank = Нужен ранг { $rank }
syndicate-untradable = Не продаётся
syndicate-per-day = { $count }/сут
syndicate-days = ≈ { $days } дн
syndicate-order = { $order ->
    [platinum] Платина
   *[other] Репутация
  }
syndicate-platinum-range = Платина
syndicate-volume-range = Оборот, в сутки
syndicate-from = от
syndicate-to = до
syndicate-name = { $tag ->
    [SteelMeridianSyndicate] Стальной Меридиан
    [ArbitersSyndicate] Арбитры Гексиса
    [CephalonSudaSyndicate] Цефалон Суда
    [PerrinSyndicate] Последовательность Перрина
    [RedVeilSyndicate] Красная Вуаль
    [NewLokaSyndicate] Новая Лока
    [LibrarySyndicate] Цефалон Симэрис
    [CetusSyndicate] Острон
    [QuillsSyndicate] Перья
    [SolarisSyndicate] Объединение Солярис
    [VentKidsSyndicate] Дети Труб
    [VoxSyndicate] Глас Солярис
    [EntratiSyndicate] Энтрати
    [NecraloidSyndicate] Некралоид
    [ZarimanSyndicate] Неукротимые
    [EntratiLabSyndicate] Кавия
    [KahlSyndicate] Гарнизон Кахла
    [HexSyndicate] Гекс
   *[other] { $tag }
  }
agent-tied = Связано
agent-today = Сегодня
agent-exchanges = Обменов
agent-warmth = Прогрев:
agent-settings = Настройки агента
policy-budget = Бюджет
policy-budget-own = Число
policy-budget-purse = Доля кошелька
policy-least-exchanges = Оставить себе обменов
policy-items = Предметам
policy-items-share = Предметам, %
policy-rivens = Ривены
policy-rivens-amount = Ривенам
policy-rivens-share = Ривенам, %
policy-riven-margin = Запас, %
policy-riven-least-profit = Мин. прибыль, пл
policy-rank-up = Прокачка модов
policy-section-items = Предметы
agent-rivens = Ривены
riven-figure-bought = Куплено
riven-figure-sold = Продано
riven-figure-profit = Прибыль
riven-figure-held = В ривенах
riven-picks = Находки
riven-picks-none = Находок нет
riven-picks-off = Ривены выключены
riven-held = Куплены
pick-reason = { $reason ->
    [unrolled] некрученый
   *[cheap] дёшево
}
pick-paid = Цена
pick-bought = Куплено
pick-dismiss = Убрать
pick-open = Открыть на warframe.market
pick-offer = Предложить
pick-seller = Продавец
pick-sold = Продан
pick-sold-for = Продан за
pick-bought-on = Куплен
pick-notice = Находка ривена
pick-notice-detail = { $name } · { $buyout } при рынке { $worth }
orders-fresh = { $secs } с

## Помощник мастерства

mastery-tab-plan = План
mastery-way-relics = Реликвии
mastery-way-unknown = источник неизвестен
mastery-standing = репутации
mastery-plan-add = В план
mastery-plan-remove = Убрать из плана
mastery-all-shelves = Все
plan-needs = Что делать
plan-need = { $need ->
    [level] Прокачать
    [forge] Кузня
    [resources] Ресурсы
    [relics] Реликвии
   *[obtain] Добыть
  }
plan-get = { $get ->
    [credits] Магазин
    [trader] Торговцы
    [lab] Лаборатория клана
    [drop] Дроп
   *[unknown] Источник неизвестен
  }
plan-empty = План пуст
plan-done = Здесь всё есть

inventory-listed = Выставлено на продажу
inventory-listed-hidden = Выставлено, но скрыто

agent-pulse = { $pulse ->
    [off] Выключен
    [blocked] Стоит
    [looking] Смотрит рынок
   *[waiting] Ждёт движения рынка
  }
agent-pace = { $count } в минуту
agent-took = взял
agent-reason = { $reason ->
    [blacklisted] в чёрном списке
    [not-whitelisted] не в белом списке
    [already-listed] уже есть ордер
    [thin-volume] мало продаж
    [too-dear] слишком дорого
    [below-floor] мала выгода
    [no-capital] не хватает бюджета
    [no-exchanges] кончились обмены
    [too-many-orders] предел ордеров
    [purse-unknown] кошелёк неизвестен
    [no-reading] нет данных рынка
    [refused] рынок отказал
    [no-seller] нет продавца в игре
    [falling] цена падает
    [no-endo] не хватает эндо
    [warming] прогрев
   *[other] { $reason }
  }
agent-stats = Статистика
agent-profit = Прибыль
agent-taken = Выручка
agent-spent = Потрачено
agent-sold = Продажи
agent-bought = Покупки
agent-sales = { $count ->
    [one] { $count } продажа
    [few] { $count } продажи
   *[many] { $count } продаж
  }
agent-buys = { $count ->
    [one] { $count } покупка
    [few] { $count } покупки
   *[many] { $count } покупок
  }
agent-margined = с { $count ->
    [one] { $count } продажи
   *[other] { $count } продаж
  }
agent-copies = { $count ->
    [one] { $count } копия
    [few] { $count } копии
   *[many] { $count } копий
  }
agent-expected = ожидается
agent-release = Забрать у агента
agent-release-confirm = Подтвердить: забрать у агента
agent-now = Сейчас
agent-period = За период
agent-in-bids = В ставках
agent-unsold = Куплено, не продано
agent-unsold-worth = Стоит сейчас
agent-free = Свободно
agent-markup = Наценка
agent-per-flip = За сделку
agent-bid-count = { $count ->
    [one] { $count } ставка
    [few] { $count } ставки
   *[many] { $count } ставок
  }
agent-by-group = По типам
agent-by-day = По дням
agent-earners = Лучшие предметы
agent-no-stats = Сделок за период нет
inventory-price = Цена
range-from = от
range-to = до
kit-parts = { $held } из { $total } частей
kit-whole = собирается ×{ $count }
kit-missing = не хватает { $count }
kit-buy-for = докупить
kit-order = { $order ->
    [ready] Ближе к комплекту
    [platinum] Дороже комплект
   *[missing] Дешевле докупить
  }
trade-edit = Изменить цену и количество
trade-add-one = Ещё один
trade-fulfil = Исполнен
trade-keep = Сохранить
trade-cancel = Отмена
trade-resale = Цель
haul-each = по { $platinum }
rivens-title = Ривены
rivens-search = Оружие или ривен
rivens-sort = Порядок
rivens-by-grade = По оценке
rivens-by-weapon = По оружию
rivens-by-rerolls = По роллам
rivens-none = Ривенов нет
rivens-veiled = Закрытые
riven-class = { $class ->
    [rifle] Винтовка
    [shotgun] Дробовик
    [pistol] Пистолет
    [melee] Ближний бой
    [archgun] Оружие арчвинга
    [companion] Оружие компаньона
    [kitgun] Китган
   *[zaw] Зо
  }
riven-mastery = МР { $mastery }
riven-rerolls = Роллы
riven-unvalued = Не оценивается
riven-grade = { $grade ->
    [perfect] Идеальный
    [good] Хороший
    [potential] С потенциалом
   *[bad] Плохой
  }
riven-verdict = { $verdict ->
    [decisive] Обязательный
    [wanted] Нужный
    [harmless] Безвредный
    [harmful] Вредный
   *[useless] Бесполезный
  }
riven-roll = Разброс { $percent }%
riven-sell = Продать
riven-price = Цена
riven-ceiling = Потолок
riven-ceiling-hint = После удачного ролла, по сделкам за неделю
riven-demand = Спрос
riven-demand-hint = По сделкам за неделю и лотам на рынке
riven-demand-value = { $demand ->
    [lively] Высокий
    [slow] Вялый
    [dead] Мёртвый
   *[unclear] Мало данных
  }
riven-reroll = Крутить
riven-transmute = В трансмутацию
riven-lock = Блок
riven-lock-none = Без блока
riven-chance = Шанс
riven-kuva = { $amount } кувы
riven-band = { $basis ->
    [lots] { $low }–{ $high } · похожих { $lots }
    [fewLots] { $low }–{ $high } · мало похожих: { $lots }
   *[trades] по сделкам за неделю
  }
riven-lots = { $lots ->
    [one] { $lots } лот
    [few] { $lots } лота
   *[many] { $lots } лотов
  } · { $basis ->
    [comparables] по аналогам
   *[curve] по кривой
  }
riven-market = На рынке
riven-rolls = Ролл
riven-better = Исход
riven-kuva-head = Кувы
riven-rolls-count = { $count ->
    [one] { $count } ролл
    [few] { $count } ролла
   *[many] { $count } роллов
  }
riven-no-analogs = Похожих лотов нет
riven-seller-status = { $status ->
    [ingame] В игре
    [online] В сети
   *[offline] Не в сети
  }
riven-sale-kind = Вид продажи
riven-sale-fixed = Фикс
riven-sale-auction = Аукцион
riven-starting = Старт
riven-buyout = Выкуп
orders-items = Предметы
orders-rivens = Ривены
auctions-none = Аукционов нет
auction-market = Рынок
auction-band = Вилка
auction-lots = Лотов
auction-from = старт { $starting }
auction-bids-only = только ставки
riven-top-bid = Ставка
