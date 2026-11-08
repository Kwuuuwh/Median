# The language name in that language.
language-name = English

## Startup screen

gate-preparing = Preparing
gate-checking = Checking for updates
gate-fetching = Downloading the catalog
gate-pictures = Downloading the pictures
gate-installing = Installing
gate-offline-title = No connection
gate-offline-hint = The catalog was not fetched
gate-offline-retry = Try again
gate-outdated-title = Update required
gate-outdated-hint = The data is newer than this build
gate-stale-title = Data older than the build
gate-stale-hint = The published catalog is not one this build reads

## Window bar

titlebar-minimize = Minimize
titlebar-maximize = Maximize
titlebar-close = Close

## Toasts

toast-dismiss = Dismiss
toast-command-failed = The command failed

## Timers

timers-title = Overview
timers-fissures = Fissures
timers-waiting = No data yet
timers-stale = updated { $minutes } min ago

## World cycles

cycle-name = { $cycle ->
    [cetus] Cetus
    [earth] Earth
    [vallis] Orb Vallis
    [cambion] Cambion Drift
    [duviri] Duviri
   *[other] { $cycle }
  }
cycle-phase = { $phase ->
    [day] day
    [night] night
    [warm] warm
    [cold] cold
    [fass] Fass
    [vome] Vome
    [joy] joy
    [anger] anger
    [envy] envy
    [sorrow] sorrow
    [fear] fear
   *[other] { $phase }
  }
# A cycle phase change.

## Fissures

era-name = { $era ->
    [lith] Lith
    [meso] Meso
    [neo] Neo
    [axi] Axi
    [requiem] Requiem
    [omnia] Omnia
    [vanguard] Vanguard
    [void] Void
   *[other] { $era }
  }
# Short marks on a fissure.
fissure-hard = SP

## Resets

reset-name = { $reset ->
    [daily] Daily reset
    [sortie] Sortie
    [weekly] Weekly reset
   *[other] { $reset }
  }

## Traders

trader-name = { $vendor ->
    [baro] Baro Ki'Teer
    [varzia] Varzia
   *[other] { $vendor }
  }
trader-here = on site
trader-road = on the road
trader-until-arrival = arrives in
trader-empty = No offers

## Trader's shelf

shelf-group-label = Kind
shelf-search = Search by name
shelf-nothing = Nothing matched

## Item reference

card-inside = Contents
card-empty = No data
card-close = Close
# $percent is a share of a hundred.
card-chance = { $percent }%
card-less = Collapse
card-more = { $count } more
# Kind of source; the key is as in the catalog.
card-source-kind = { $kind ->
    [relic] Relics
    [node] Nodes
    [bounty] Bounties
    [key] Keys
    [transient] Transient
    [sortie] Sortie
    [enemy] Enemies
    [event] Events
    [vendor] Sellers
    [lab] Labs
   *[other] { $kind }
  }
card-rotation = rotation { $rotation }
card-rank = rank { $rank }

## Holdings

inventory-title = Inventory
inventory-waiting = No holdings data

## Toolbar

sort-direction = { $direction ->
    [up] Ascending
   *[down] Descending
  }

## Filters

filter-shown = { $shown } of { $count } shown
filter-class-any = All classes

## The wall

price-age-fresh = just now
price-age-minutes = { $count } min
price-age-hours = { $count } h
price-age-days = { $count } d

## Trade panel

deal-orders-asking = Loading orders
deal-status-ingame = In game
deal-status-online = Online
deal-status-offline = Offline
deal-orders-none = No orders
orders-rank-pick = { $rank ->
    [max] Rank { $max }
   *[zero] Rank 0
  }
deal-price = Price

## Price chart

chart-closed = Trades

## Vocabularies

rarity-name = { $rarity ->
    [common] common
    [uncommon] uncommon
    [rare] rare
   *[other] { $rarity }
  }
# Currency; the key is as in the core.
currency-short = { $currency ->
    [ducats] duc
    [credits] cr
    [aya] aya
    [regalAya] regal aya
   *[other] { $currency }
  }
# A seller's currency; the key is the catalog's name in lower-case words joined by dashes.
vendor-currency = { $currency ->
    [animo-nav-beacon] Animo Nav Beacon
    [aya] Aya
    [beating-heartstrings] Beating Heartstrings
    [corrupted-holokey] Corrupted Holokey
    [cred] Cred
    [credits] Credits
    [dominus-aureus] Dominus Aureus
    [ducats] Ducats
    [enigma-gyrum] Enigma Gyrum
    [fate-pearl] Fate Pearl
    [fergolyte] Fergolyte
    [grandmother-token] Grandmother Token
    [kullervos-bane] Kullervo's Bane
    [lua-thrax-plasm] Lua Thrax Plasm
    [maphica] Maphica
    [mother-token] Mother Token
    [nakak-pearl] Nakak Pearl
    [pathos-clamp] Pathos Clamp
    [phasic-cells] Phasic Cells
    [pix-chip] Pix Chip
    [platinum] Platinum
    [riven-sliver] Riven Sliver
    [scarlet-credits] Scarlet Credits
    [scuttler-husk] Scuttler Husk
    [standing] Standing
    [steel-essence] Steel Essence
    [stock-kahl] Stock (Kahl)
    [vessel-capillaries] Vessel Capillaries
    [vestigial-motes] Vestigial Motes
    [vitus-essence] Vitus Essence
    [voidplume-pinion] Voidplume Pinion
    [volatile-motes] Volatile Motes
   *[other] { $name }
  }
# What a variant says about a copy.
detail-rank = rank { $rank }
detail-stars = { $stars ->
    [one] { $stars } star
   *[other] { $stars } stars
  }
detail-veiled = veiled

## Settings

settings-title = Settings

settings-general = General
settings-general-note = Language and notifications
settings-appearance = Appearance
settings-appearance-note = Highlight colour
settings-updates = Updates
settings-updates-note = App version and updates
settings-build = Build
settings-channel = Channel
release-channel = { $channel ->
    [stable] stable
    [preview] preview
   *[dev] dev
  }
settings-channel-hint = preview gets changes before stable
settings-channel-local = A local build does not update
settings-log = Log
settings-log-hint = The median.log file to attach to a bug report
settings-log-open = Open folder
settings-data = Game data
settings-data-date = from { $date }
settings-data-game = Game build { $build }
settings-changes = What changed in this version
settings-changes-local = A local build has no change list
settings-changes-none = No changes recorded for this version
# $revision is "r8217".
update-checking = Checking for a new version
update-skip = Skip
update-unchecked = Not checked yet
update-latest = This is the latest version
update-waiting = Waiting for { $channel } newer than { $revision }
update-check = Check
update-check-failed = Check failed: { $reason }
update-found = { $revision } is available
update-later = Later
update-install = Update
update-downloading = Downloading { $revision }
update-downloading-share = Downloading { $revision }: { $percent }%
update-installing = Installing
update-installing-hint = The app will close and start again
update-failed = Update failed
update-install-failed = Update failed: { $reason }
update-continue = Continue
settings-interface-language = Interface language
settings-language-hint = Interface and item names from the catalog
settings-whisper-sound = Sound on a whisper
settings-whisper-sound-hint = When someone messages you in the game
settings-notifications = Notifications

settings-language = Language

settings-accent = Accent
# Accent names; the key is what the core calls the accent.
accent-name = { $accent ->
    [ice] Ice
    [indigo] Indigo
    [amethyst] Amethyst
    [orchid] Orchid
    [steel] Steel
    [terracotta] Terracotta
   *[other] { $accent }
  }

settings-build-version = Version

account-email = Email
account-password = Password
account-sign-in = Sign in
account-sign-out = Sign out
account-expired = The session expired, sign in again
status-auto = Auto
status-title = Market status
status-online = Online
status-ingame = In game
status-invisible = Invisible
status-out = Signed out

orders-title = Orders
orders-selling = Selling
orders-buying = Buying
orders-edit-save = Save
orders-none = No orders
orders-loading = Updating orders

policy-capital = Capital
policy-most-per-order = Copies in one bid
policy-a-trade = Least per flip

figures-group = { $group ->
    [riven] Rivens
    [relic] Relics
    [arcane] Arcanes
    [mod] Mods
    [set] Sets
    [prime] Prime
   *[other] Other
  }
whisper-writes = { $player } is writing
alert-volume = Volume

deal-record-buy = Bought
deal-record-sell = Sold
deal-record-price = Price a copy
deal-record-quantity = Quantity
deal-record-save = Record
hand-to-agent = Hand to the agent
hand-to-agent-title = Hand to the agent
hand-to-agent-note = The agent offers the copies for sale and never asks less than they cost.
hand-to-agent-price = Paid a copy
hand-to-agent-quantity = Quantity
hand-to-agent-save = Hand over

# The layer drawn over the game.
overlay-whole = set
overlay-built = built
overlay-mastered = mastered
overlay-updating = updating
overlay-per-day = { $count }/day
riven-cycle-reading = Recognition
riven-cycle-unread = { $why ->
    [noReader] Recognition model did not load
    [notHeld] Riven not in the inventory
    [unsupported] Weapon not supported
   *[stats] Stats not recognised
  }
riven-cycle-pricing = Pricing
riven-cycle-appraising = Appraisal

## Relics

relics-title = Relics
relics-search = Relic or reward
relics-era = Era
relics-any-era = All
relics-squad = Squad
relics-sort = Order
relics-by-platinum = By platinum
relics-by-ducats = By ducats
relics-by-upgrade = By refining
relics-by-count = By count
relics-none = Nothing found
relic-vaulted = vaulted
relic-refinement = { $refinement ->
    [intact] Intact
    [exceptional] Exceptional
    [flawless] Flawless
    [radiant] Radiant
   *[other] { $refinement }
  }

## Mastery and catalogue

mastery-title = Mastery
catalogue-title = Catalogue

span-days = { $days } d
span-days-hours = { $days } d { $hours } h
span-hours-minutes = { $hours } h { $minutes } min
span-minutes = { $minutes } min

mastery-rank-label = rank
mastery-progress = { $total } / { $next }
mastery-progress-floor = ≥ { $total } / { $next }
mastery-remaining = { $remaining } to go
mastery-test-open = test open
mastery-test-shut = test in { $left }
mastery-undercounted = below the rank's threshold
mastery-uncounted = { $source ->
    [items] some items left out
    [nodes] nodes left out
   *[other] { $source }
  }
mastery-source = { $source ->
    [items] Items
    [nodes] Star chart
    [steelPath] Steel Path
    [junctions] Junctions
    [steelJunctions] Steel Path junctions
    [railjack] Railjack
    [drifter] Drifter
   *[other] { $source }
  }
mastery-share = { $done } / { $of }
mastery-search = Item
mastery-shelf = { $shelf ->
    [warframe] Warframes
    [primary] Primary
    [secondary] Secondary
    [melee] Melee
    [modular] Modular
    [archwing] Archwing
    [companion] Companions
    [companion-weapon] Companion weapons
   *[other] { $shelf }
  }
mastery-gain = +{ $points }
mastery-gain-rank = +{ $points }, rank { $rank }
mastery-view-items = Items
mastery-in-hand = { $points } in hand
rank-gate = Mastery rank { $rank }
mastery-none = Nothing found
mastery-sources = Mastery sources

roadmap-clear = Clear
roadmap-clear-confirm = Sure?
chain-ready = Can build
chain-everywhere = Everywhere
roadmap-sum-gain = Mastery
roadmap-gain = +{ $gain }
roadmap-gain-rank = enough for rank { $rank }
roadmap-sum-credits = Credits
roadmap-sum-time = In the foundry

catalogue-search = Search catalogue
catalogue-class = Class
catalogue-ready = can be built
catalogue-none = Nothing found

nav-trading = Trading
nav-collapse = Collapse sidebar
nav-expand = Expand sidebar
inventory-search = Search inventory
inventory-nothing-found = Nothing found
own-selling = Selling
own-buying = Buying
nav-world = World
timers-cycles = Cycles
fissure-plain = Normal
fissure-hard-long = Steel Path
fissure-all = All
fissure-railjack = Railjack
trader-rotation = Offer changes
alarms-title = Notifications
alarms-none = Nothing yet
alarms-clear = Clear
alarms-dismiss = Dismiss
alarms-watches = Subscriptions
alarms-no-watches = No subscriptions
alarms-drop = Unsubscribe
watch-phase = Notify on phase change
watch-arrival = Notify on arrival
fissure-watch-plain = normal
fissure-watch-hard = Steel Path
fissure-watch-storm = Railjack
alarm-arrival = arrival
alarm-rotation = new offer
mission-type = { $mission ->
    [MT_ALCHEMY] Alchemy
    [MT_ARMAGEDDON] Void Armageddon
    [MT_ARTIFACT] Disruption
    [MT_ASSAULT] Assault
    [MT_CAPTURE] Capture
    [MT_CORRUPTION] Void Flood
    [MT_DEFENSE] Defense
    [MT_EXCAVATE] Excavation
    [MT_EXTERMINATION] Extermination
    [MT_HIVE] Hive
    [MT_INTEL] Spy
    [MT_MOBILE_DEFENSE] Mobile Defense
    [MT_RESCUE] Rescue
    [MT_SABOTAGE] Sabotage
    [MT_SURVIVAL] Survival
    [MT_TERRITORY] Interception
    [MT_VOID_CASCADE] Void Cascade
   *[other] { $mission }
  }
watch-any-era = Any era
watch-any-mission = Any mission
watch-take = Subscribe
timers-resets = Resets
timers-rotations = Rotations
shelf-tab-warframes = Warframes
shelf-tab-weapons = Weapons
shelf-tab-mods = Mods
shelf-tab-relics = Relics
shelf-tab-cosmetics = Cosmetics
shelf-tab-other = Other
shelf-tab-bundles = Bundles
shelf-tab-regal = Regal Aya
item-held = owned ×{ $count }
item-mastered = mastered
item-favourite = Favourite
shelf-tab-primes = Primes
shelf-per-run = p per run
shelf-no-drops = not in relics
shelf-sort = Order
shelf-sort-payback = Payback
item-trades = { $count } { $count ->
    [one] trade
   *[other] trades
  }
item-whole = Set
item-sources = Source
item-daily = Trades a day
item-range = Range
item-period = Period
item-days = { $days } d
item-runs = ≈ { $runs }
item-copy = Copy message
item-refinement = Refinement
item-untraded = Not traded
item-no-trades = No trades
shelf-missing-only = Not owned
shelf-all = All
order-status = Status
order-player = Player
order-reputation = Reputation
order-quantity = Qty
order-copied = Copied
fissure-reset = Reset
shelf-unknown = Not in catalog
shelf-sort-top = Top reward
shelf-sort-held = Owned
orders-failed = The market did not answer
orders-retry = Retry
inventory-tab = { $tab ->
    [parts] Parts
    [mods] Mods
    [arcanes] Arcanes
    [sets] Sets
   *[other] Other
  }
inventory-order = { $order ->
    [platinum] Platinum
    [ducats] Ducats
    [count] Count
    [rank] Rank
   *[other] Name
  }
inventory-collections = All
inventory-uncollected = Other
inventory-rank = { $rank }/{ $max }
inventory-to-max = { $count ->
    [0] enough
   *[other] { $count } more
}
inventory-equipped = equipped
inventory-free = { $count } free
inventory-fitted = on { $count }
inventory-more = and { $count } more
item-crafted = built
facet-title = Filters
facet-built = Mastered
facet-unbuilt = Not mastered
facet-many = More than one
facet-single = One
facet-vaulted = Vaulted
facet-unvaulted = Not vaulted
facet-listed = Order up
facet-unlisted = No order
facet-prime = Prime
facet-plain = Regular
facet-favourite = Favourite
facet-unfavoured = Not favourite
facet-equipped = Equipped
facet-free = Free
facet-upgraded = Upgraded
facet-unranked = Not upgraded
facet-spare = Surplus
facet-unspared = No surplus
facet-tradable = Tradable
facet-untradable = Not tradable
facet-held = In inventory
facet-unheld = Not in inventory
facet-mastered = Mastered
facet-unmastered = Not mastered
facet-planned = In the plan
facet-unplanned = Not in the plan
facet-reachable = Rank is enough
facet-unreachable = Rank too low
facet-ready = Ready to build
facet-unready = Not ready to build
facet-relics = From own relics
facet-no-relics = Not from own relics
facet-opened = Opened by rank
facet-closed = Closed by rank
facet-affordable = Standing suffices
facet-unaffordable = Standing short
facet-beaten = Beaten
facet-best = Best
facet-visible = Visible
facet-hidden = Hidden
facet-agent = By the agent
facet-mine = Mine
sell-quantity = Quantity
sell-price = Price
sell-place = List
sell-placed = Listed
sell-signed-out = Not signed in to warframe.market
order-lot = { $count } per trade · { $sum }
sell-rank = Rank
sell-per-trade = Per trade
sell-held = { $count } held
troubles-title = Troubles
trouble-gep-outdated = Collector of another version
trouble-dbwin-closed = Game stream not read
trouble-dbwin-shared = Another application reads the game stream
trouble-task-restarting = { $name }: restarting
trouble-task-stopped = { $name }: stopped
trouble-wfm-silent = Market not answering
trouble-gep-refused = Overwolf withholds the game data
collector-folder-open = Open the collector folder
trouble-collector-missing = Collector not connected
trouble-ocr-failed = Reward recognition not working
trouble-since = since { $time }
trade-positions = Positions
trade-by-agent = Placed by the agent
trade-price = Price
trade-reference = Best
trade-median = Median
trade-quantity = Qty
trade-held = Held
trade-most = Ceiling
trade-hide = Hide
trade-show = Show
trade-more = More
trade-withdraw = Withdraw
trade-basis = Bought
trade-ingame = Market
trade-asking = Our order
trade-order = { $order ->
    [gap] Gap
    [worth] Total
    [price] Price
    [quantity] Quantity
   *[other] Name
  }
trade-refresh = Refresh from the market
trade-show-all = Show all
trade-hide-all = Hide all
trade-withdraw-all = Take all down: { $count }
trade-withdraw-all-confirm = Confirm taking down: { $count }
trade-no-positions = No positions
trade-period = { $days ->
    [0] All
   *[other] { $days } d
  }
agent-name = Agent
agent-bids = Bids
agent-no-bids = No bids
agent-blocker = { $blocker ->
    [signed-out] Not signed in to WFM
    [purse] Purse not read
    [budget] Budget 0
   *[exchanges] No exchanges left
  }
haul-title = Session
haul-continue = Continue
haul-close = End session
syndicates-title = Syndicates
syndicate-rank = Rank { $rank }
syndicate-standing = { $tag ->
    [KahlSyndicate] Stock
   *[other] Standing
  }
syndicate-today = Left today
syndicates-unread = No standing read yet
syndicates-all = All syndicates
syndicate-offers = { $count } for sale
syndicate-unmet = Not met
syndicate-needs-rank = Rank { $rank } needed
syndicate-untradable = Not tradable
syndicate-per-day = { $count }/day
syndicate-days = ≈ { $days } d
syndicate-order = { $order ->
    [platinum] Platinum
   *[other] Standing
  }
syndicate-platinum-range = Platinum
syndicate-volume-range = Turnover, a day
syndicate-from = from
syndicate-to = to
syndicate-name = { $tag ->
    [SteelMeridianSyndicate] Steel Meridian
    [ArbitersSyndicate] Arbiters of Hexis
    [CephalonSudaSyndicate] Cephalon Suda
    [PerrinSyndicate] The Perrin Sequence
    [RedVeilSyndicate] Red Veil
    [NewLokaSyndicate] New Loka
    [LibrarySyndicate] Cephalon Simaris
    [CetusSyndicate] Ostrons
    [QuillsSyndicate] The Quills
    [SolarisSyndicate] Solaris United
    [VentKidsSyndicate] Ventkids
    [VoxSyndicate] Vox Solaris
    [EntratiSyndicate] Entrati
    [NecraloidSyndicate] Necraloid
    [ZarimanSyndicate] The Holdfasts
    [EntratiLabSyndicate] Cavia
    [KahlSyndicate] Kahl's Garrison
    [HexSyndicate] The Hex
   *[other] { $tag }
  }
agent-tied = Tied up
agent-today = Today
agent-exchanges = Exchanges
agent-warmth = Warm-up:
agent-settings = Agent settings
policy-budget = Budget
policy-budget-own = Number
policy-budget-purse = Share of the purse
policy-least-exchanges = Exchanges kept for yourself
policy-items = Items
policy-items-share = Items, %
policy-rivens = Rivens
policy-rivens-amount = Rivens
policy-rivens-share = Rivens, %
policy-riven-margin = Margin, %
policy-riven-least-profit = Least profit, pl
policy-rank-up = Rank mods up
policy-section-items = Items
agent-rivens = Rivens
riven-figure-bought = Bought
riven-figure-sold = Sold
riven-figure-profit = Profit
riven-figure-held = In rivens
riven-picks = Picks
riven-picks-none = No picks
riven-picks-off = Rivens are off
riven-held = Bought
pick-reason = { $reason ->
    [unrolled] unrolled
   *[cheap] cheap
}
pick-paid = Price
pick-bought = Bought
pick-dismiss = Dismiss
pick-open = Open on warframe.market
pick-offer = Offer
pick-seller = Seller
pick-sold = Sold
pick-sold-for = Sold for
pick-bought-on = Bought
pick-notice = Riven pick
pick-notice-detail = { $name } · { $buyout } against { $worth }
orders-fresh = { $secs } s

## Mastery helper

mastery-tab-plan = Plan
mastery-way-relics = Relics
mastery-way-unknown = no known source
mastery-standing = standing
mastery-plan-add = Add to plan
mastery-plan-remove = Remove from plan
mastery-all-shelves = All
plan-needs = What to do
plan-need = { $need ->
    [level] Level
    [forge] Foundry
    [resources] Resources
    [relics] Relics
   *[obtain] Obtain
  }
plan-get = { $get ->
    [credits] Market
    [trader] Traders
    [lab] Clan lab
    [drop] Drops
   *[unknown] Source unknown
  }
plan-empty = The plan is empty
plan-done = Nothing left here

inventory-listed = Listed for sale
inventory-listed-hidden = Listed, but hidden

agent-pulse = { $pulse ->
    [off] Off
    [blocked] Held back
    [looking] Watching the market
   *[waiting] Waiting for the market
  }
agent-pace = { $count } a minute
agent-took = took
agent-reason = { $reason ->
    [blacklisted] blacklisted
    [not-whitelisted] not whitelisted
    [already-listed] already listed
    [thin-volume] trades too rarely
    [too-dear] too dear
    [below-floor] margin too thin
    [no-capital] out of budget
    [no-exchanges] out of exchanges
    [too-many-orders] order limit
    [purse-unknown] purse unknown
    [no-reading] no market reading
    [refused] market refused
    [no-seller] no seller in game
    [falling] price falling
    [no-endo] not enough endo
    [warming] warming up
   *[other] { $reason }
  }
agent-stats = Statistics
agent-profit = Profit
agent-taken = Revenue
agent-spent = Spent
agent-sold = Sales
agent-bought = Purchases
agent-sales = { $count ->
    [one] { $count } sale
   *[other] { $count } sales
  }
agent-buys = { $count ->
    [one] { $count } purchase
   *[other] { $count } purchases
  }
agent-margined = from { $count ->
    [one] { $count } sale
   *[other] { $count } sales
  }
agent-copies = { $count ->
    [one] { $count } copy
   *[other] { $count } copies
  }
agent-expected = expected
agent-release = Take from the agent
agent-release-confirm = Confirm taking it from the agent
agent-now = Now
agent-period = Over the period
agent-in-bids = In bids
agent-unsold = Bought, not sold
agent-unsold-worth = Worth now
agent-free = Free
agent-markup = Markup
agent-per-flip = Per flip
agent-bid-count = { $count ->
    [one] { $count } bid
   *[other] { $count } bids
  }
agent-by-group = By type
agent-by-day = By day
agent-earners = Best items
agent-no-stats = No trades in this period
inventory-price = Price
range-from = from
range-to = to
kit-parts = { $held } of { $total } parts
kit-whole = makes ×{ $count }
kit-missing = { $count } missing
kit-buy-for = buy for
kit-order = { $order ->
    [ready] Nearest to whole
    [platinum] Dearest set
   *[missing] Cheapest to finish
  }
trade-edit = Change price and quantity
trade-add-one = One more
trade-fulfil = Fulfilled
trade-keep = Keep
trade-cancel = Cancel
trade-resale = Target
haul-each = at { $platinum }
rivens-title = Rivens
rivens-search = Weapon or riven
rivens-sort = Order
rivens-by-grade = By grade
rivens-by-weapon = By weapon
rivens-by-rerolls = By rolls
rivens-none = No rivens
rivens-veiled = Veiled
riven-class = { $class ->
    [rifle] Rifle
    [shotgun] Shotgun
    [pistol] Pistol
    [melee] Melee
    [archgun] Archgun
    [companion] Companion weapon
    [kitgun] Kitgun
   *[zaw] Zaw
  }
riven-mastery = MR { $mastery }
riven-rerolls = Rolls
riven-unvalued = Not valued
riven-grade = { $grade ->
    [perfect] Perfect
    [good] Good
    [potential] Has potential
   *[bad] Bad
  }
riven-verdict = { $verdict ->
    [decisive] Required
    [wanted] Wanted
    [harmless] Harmless
    [harmful] Harmful
   *[useless] Useless
  }
riven-roll = Roll { $percent }%
riven-sell = Sell
riven-price = Price
riven-ceiling = Ceiling
riven-ceiling-hint = After a lucky roll, by last week's trades
riven-demand = Demand
riven-demand-hint = By last week's trades and lots listed
riven-demand-value = { $demand ->
    [lively] High
    [slow] Slow
    [dead] Dead
   *[unclear] Too little data
  }
riven-reroll = Roll
riven-transmute = Transmute
riven-lock = Lock
riven-lock-none = No lock
riven-chance = Chance
riven-kuva = { $amount } kuva
riven-band = { $basis ->
    [lots] { $low }–{ $high } · { $lots } alike
    [fewLots] { $low }–{ $high } · few alike: { $lots }
   *[trades] by last week's trades
  }
riven-lots = { $lots ->
    [one] { $lots } lot
   *[other] { $lots } lots
  } · { $basis ->
    [comparables] by alike
   *[curve] by curve
  }
riven-market = On the market
riven-rolls = Roll
riven-better = Outcome
riven-kuva-head = Kuva
riven-rolls-count = { $count ->
    [one] { $count } roll
   *[other] { $count } rolls
  }
riven-no-analogs = No lots like it
riven-seller-status = { $status ->
    [ingame] In game
    [online] Online
   *[offline] Offline
  }
riven-sale-kind = Sale
riven-sale-fixed = Fixed
riven-sale-auction = Auction
riven-starting = Starting
riven-buyout = Buyout
orders-items = Items
orders-rivens = Rivens
auctions-none = No auctions
auction-market = Market
auction-band = Band
auction-lots = Lots
auction-from = from { $starting }
auction-bids-only = bids only
riven-top-bid = Top bid
