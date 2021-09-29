from collections import OrderedDict

POTENTIALLY_REQUIRED_DUNGEONS = [
    "Skyview",
    "Earth Temple",
    "Lanayru Mining Facility",
    "Ancient Cistern",
    "Sandship",
    "Fire Sanctuary",
]

DUNGEON_NAMES = OrderedDict(
    [
        ("SV", "Skyview"),
        ("ET", "Earth Temple"),
        ("LMF", "Lanayru Mining Facility"),
        ("AC", "Ancient Cistern"),
        ("SS", "Sandship"),
        ("FS", "Fire Sanctuary"),
        ("SK", "Sky Keep"),
        ("LanayruCaves", "Lanayru Caves"),  # "short name" doesn't allow space
    ]
)
DUNGEON_NAME_TO_SHORT_DUNGEON_NAME = OrderedDict(
    [v, k] for k, v in DUNGEON_NAMES.items()
)

ENTRANCE_CONNECTIONS = {
    "Dungeon Entrance In Deep Woods": "Skyview",
    "Dungeon Entrance In Eldin Volcano": "Earth Temple",
    "Dungeon Entrance In Lanayru Desert": "Lanayru Mining Facility",
    "Dungeon Entrance In Lake Floria": "Ancient Cistern",
    "Dungeon Entrance In Sand Sea": "Sandship",
    "Dungeon Entrance In Volcano Summit": "Fire Sanctuary",
    "Dungeon Entrance On Skyloft": "Sky Keep",
}

SILENT_REALMS = OrderedDict(
    [
        ("Skyloft Silent Realm", "Skyloft Trial Gate"),
        ("Faron Silent Realm", "Faron Trial Gate"),
        ("Lanayru Silent Realm", "Lanayru Trial Gate"),
        ("Eldin Silent Realm", "Eldin Trial Gate"),
    ]
)

SILENT_REALM_CHECKS = OrderedDict(
    [
        ("Skyloft Silent Realm - Stone of Trials", "Trial Gate on Skyloft"),
        ("Faron Silent Realm - Water Scale", "Trial Gate in Faron Woods"),
        ("Lanayru Silent Realm - Clawshots", "Trial Gate in Lanayru Desert"),
        ("Eldin Silent Realm - Fireshield Earrings", "Trial Gate in Eldin Volcano"),
    ]
)

SHOP_CHECKS = [
    "Beedle - 50 Rupee Item",
    "Beedle - First 100 Rupee Item",
    "Beedle - Second 100 Rupee Item",
    "Beedle - Third 100 Rupee Item",
    "Beedle - 300 Rupee Item",
    "Beedle - 600 Rupee Item",
    "Beedle - 800 Rupee Item",
    "Beedle - 1000 Rupee Item",
    "Beedle - 1200 Rupee Item",
    "Beedle - 1600 Rupee Item",
]

MAP_CHECKS = [
    "Skyview - Map Chest",
    "Earth Temple - Map Chest",
    "Lanayru Mining Facility - Map Chest",
    "Ancient Cistern - Map Chest",
    "Sandship - Map Chest",
    "Fire Sanctuary - Map Chest",
    "Sky Keep - Map Chest",
]

SMALL_KEY_CHECKS = [
    "Skyview - Behind Two Eyes",
    "Skyview - Behind Three Eyes",
    "Lanayru Mining Facility - First Chest in Hub Room",
    "Ancient Cistern - Small Key Chest",
    "Ancient Cistern - Bokoblin",
    "Sandship - Behind Combination Lock",
    "Sandship - Robot in Brig",
    "Fire Sanctuary - First Room",
    "Fire Sanctuary - Second Small Key Chest",
    "Fire Sanctuary - Third Small Key Chest",
    "Lanayru Caves - Golo",
    "Sky Keep - Small Key Chest",
]

BOSS_KEY_CHECKS = [
    "Skyview - Boss Key",
    "Earth Temple - Boss Key",
    "Lanayru Mining Facility - Boss Key",
    "Ancient Cistern - Boss Key",
    "Sandship - Boss Key",
    "Fire Sanctuary - Boss Key",
]

END_OF_DUNGEON_CHECKS = OrderedDict(
    [
        ("Skyview", "Skyview - Ruby Tablet"),
        ("Earth Temple", "Earth Temple - Amber Tablet"),
        ("Lanayru Mining Facility", "Lanayru Mining Facility - Harp"),
        ("Ancient Cistern", "Ancient Cistern - Goddess Longsword"),
        ("Sandship", "Sandship - Nayru's Flame"),
        ("Fire Sanctuary", "Fire Sanctuary - Din's Flame"),
    ]
)

STARTING_SWORD_COUNT = {
    "Swordless": 0,
    "Practice Sword": 1,
    "Goddess Sword": 2,
    "Goddess Longsword": 3,
    "Goddess White Sword": 4,
    "Master Sword": 5,
    "True Master Sword": 6,
}

ALL_TYPES = [
    "skyloft",
    "sky",
    "thunderhead",
    "faron",
    "eldin",
    "lanayru",
    "dungeon",
    "mini dungeon",
    "free gift",
    "freestanding",
    "miscellaneous",
    "silent realm",
    "digging",
    "bombable",
    "combat",
    "song",
    "spiral charge",
    "minigame",
    "crystal",
    "short",
    "long",
    "fetch",
    "crystal quest",
    "scrapper",
    "peatrice",
    "beedle",
    "cheap",
    "medium",
    "expensive",
    "goddess",
    "faron goddess",
    "eldin goddess",
    "lanayru goddess",
    "floria goddess",
    "summit goddess",
    "sand sea goddess",
]

POTENTIALLY_BANNED_TYPES = {
    "dungeon": "Dungeons",
    "mini dungeon": "Mini Dungeons",
    "free gift": "Free Gifts",
    "freestanding": "Freestanding Items",
    "miscellaneous": "Miscellaneous",
    "silent realm": "Silent Realms",
    "digging": "Digging Spots",
    "bombable": "Bombable Walls",
    "combat": "Combat Rewards",
    "song": "Songs",
    "spiral charge": "Spiral Charge Chests",
    "minigame": "Minigames",
    "short": "Short Quests",
    "long": "Long Quests",
    "fetch": "Fetch Quests",
    "crystal quest": "Gratitude Crystal Quests",
    "scrapper": "Scrapper Quests",
    "peatrice": "Peatrice",
    "beedle": "Beedle's Shop",
    "cheap": "Cheap Purchases in Beedle's Shop",
    "medium": "Medium Purchases in Beedle's Shop",
    "expensive": "Expensive Purchases in Beedle's Shop",
    "goddess": "All Goddess Chests",
    "faron goddess": "Goddess Cubes in Faron Woods",
    "eldin goddess": "Goddess Cubes in Eldin Volcano",
    "lanayru goddess": "Goddess Cubes in Lanayru Desert",
    "floria goddess": "Goddess Cubes in Lake Floria",
    "summit goddess": "Goddess Cubes in Volcano Summit",
    "sand sea goddess": "Goddess Cubes in Lanayru Sand Sea",
}

NON_RANDOMIZED_SETTINGS = [
    "No Spoiler Log",
    "Fix BiT crashes",
    "Randomize Settings",
    "Banned Types",  # done separately
    # Currently unavailable
    "Gate of Time Starting State",
    "Gate of Time Dungeon Requirements",
    "Horde",
    "Ghirahim 3",
    "Demise",
    # Unrandomized
    "Logic Mode",
    "Enabled Tricks BiTless",
    "Enabled Tricks Glitched",
    # Hints (done separately)
    "Hint Distribution",
    "Way of the Hero Hints",
    "Sometimes Hints",
    "Barren",
    "Location Hints",
    "Item Hints",
]

HINT_SETTINGS = [
    "Hint Distribution",
    "Way of the Hero Hints",
    "Sometimes Hints",
    "Barren",
    "Location Hints",
    "Item Hints",
]
