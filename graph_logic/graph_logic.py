from typing import Dict, List, Set, DefaultDict
from collections import defaultdict
from random import Random
from .dataparse import parse_all
from options import Options
from .logic_expression import LogicState
from .logic_types import Area, Areakey, Checkname, ForceTod
from logic.item_types import (
    PROGRESS_ITEMS,
    NONPROGRESS_ITEMS,
    CONSUMABLE_ITEMS,
    DUPLICATABLE_CONSUMABLE_ITEMS,
    DUNGEON_PROGRESS_ITEMS,
    DUNGEON_NONPROGRESS_ITEMS,
    SMALL_KEYS,
    BOSS_KEYS,
)
from logic.constants import (
    DUNGEON_NAME_TO_SHORT_DUNGEON_NAME,
    DUNGEON_NAMES,
    SHOP_CHECKS,
    MAP_CHECKS,
    SMALL_KEY_CHECKS,
    BOSS_KEY_CHECKS,
    END_OF_DUNGEON_CHECKS,
    POTENTIALLY_REQUIRED_DUNGEONS,
    ALL_TYPES,
    STARTING_SWORD_COUNT,
)
from util.file_accessor import read_yaml_file_cached


class FillException(Exception):
    pass


batreaux_location_re = re.compile(r".*Batreaux - ([0-9]+) .*")


class GraphLogic:
    def __init__(self, areas: List[Area], options: Options):
        self.areas: Dict[Areakey, Area] = dict()
        for area in areas:
            self.areas[area.get_areakey()] = area
        self.options = options
        self.macros: Dict[str, "LogicExpression"] = {}
        self.loc_to_area: Dict[Checkname, Areakey] = dict()
        for area in self.areas.values():
            for loc in area.locations:
                self.loc_to_area[loc] = area.get_areakey()

        self.checks = read_yaml_file_cached("checks.yaml")
        # all locations that can hold progress items
        self.progress_locations: List[Checkname] = []
        # all locations that can not hold progress items
        # every locations is either in the progress list or the non progress list
        self.non_progress_locations: List[Checkname] = []
        # items you start with
        self.start_items: List[str] = []
        # items that can open up locations
        # and should appear in progress locations
        self.progress_items: List[str] = PROGRESS_ITEMS + DUNGEON_PROGRESS_ITEMS
        # items that can't open up locations, but should
        # appear in progress locations
        self.useful_items: List[str] = []
        # items that can't open up locations, can appear
        # in any locations
        self.non_progress_items: List[str] = (
            NONPROGRESS_ITEMS + DUNGEON_NONPROGRESS_ITEMS
        )
        # items that might not all be placed, can appear
        # in any locations
        self.collectible_items: List[str] = []

        self.hintbanned_locations: Set[str] = set()

        # set up progress/non progress locations
        # ban areas, then do exploration and all non reachable
        # are non progress
        for locname, loc in self.checks.items():
            # types
            if loc["type"].isdisjoint(self.options["banned-types"]):
                # batreaux
                bat_loc_match = batreaux_location_re.match(location_name)
                if bat_loc_match:
                    if self.options["max-batreaux-reward"] < int(
                        bat_loc_match.group(1)
                    ):
                        self.racemode_ban_location(location_name)

        # logic var macros, for required dungeons

    def is_progress_location(self, locname) -> bool:
        if not self.checks[locname]["type"].isdisjoint(self.options["banned-types"]):
            return False
        if loc["type"].isdisjoint(self.options["banned-types"]):
            # batreaux
            bat_loc_match = batreaux_location_re.match(location_name)
            if bat_loc_match:
                if self.options["max-batreaux-reward"] < int(bat_loc_match.group(1)):
                    return False

    # this function is guaranteed to give the same output given the same input
    # ordering of `filled_locations` doesn't matter
    # tries to place all items from `items_to_place` into the locations `locations_to_fill`
    # drains both lists
    def fill_assumed(
        self,
        filled_locations: Dict[Checkname, str],
        assumed_items: List[str],
        items_to_place: List[str],
        locations_to_fill: List[Checkname],
        rng: Random,
        mark_as_hintbanned: bool = False,
    ):
        rng.shuffle(locations_to_fill)
        rng.shuffle(items_to_place)

        while items_to_place and locations_to_fill:
            current_item = items_to_place.pop()
            # print(f"placing {current_item}")
            logic_state = LogicState(self.options, self.macros)
            logic_state.owned_areas.add(Areakey("Knight Academy", "Main"))
            logic_state.owned_events.update(
                (
                    "Sealed Grounds Statue",
                    "Eldin Entrance Statue",
                    "Lanayru Mine Entry Statue",
                )
            )
            for item in assumed_items:
                logic_state.collect_item(item)
            for item in items_to_place:
                logic_state.collect_item(item)
            self.do_exploration(logic_state, filled_locations)
            could_be_placed = False
            for loc in locations_to_fill:
                area = self.loc_to_area[loc]
                if area in logic_state.owned_areas:
                    req = self.areas[area].locations[loc]
                    if req.is_true(
                        logic_state, logic_state.options, logic_state.macros
                    ):
                        locations_to_fill.remove(loc)
                        filled_locations[loc] = current_item
                        could_be_placed = True
                        if mark_as_hintbanned:
                            self.hintbanned_locations.add(loc)
                        # print(f"placed {current_item} at {loc}")
                        break
                    # print(f"failed to place at {loc}")
            if not could_be_placed:
                items_to_place.append(item)
                print(filled_locations)
                raise FillException(f"no open locations for {item}!")

    # checks all areas for exits, updates logic_state with those new areas
    # returns if new areas could be reached
    def _exploration_subroutine(self, logic_state: LogicState) -> bool:
        checked_areas: Set[Areakey] = set()
        areas_to_check: Set[Areakey] = set(logic_state.owned_areas)
        reached_new = False
        while areas_to_check:
            areakey = areas_to_check.pop()
            checked_areas.add(areakey)
            area = self.areas[areakey]
            for logic_exit, req in area.logic_exits.items():
                if logic_exit not in checked_areas and logic_exit not in areas_to_check:
                    if req.is_true(
                        logic_state, logic_state.options, logic_state.macros
                    ):
                        # print(f"now reached {logic_exit}")
                        reached_new = True
                        areas_to_check.add(logic_exit)
                        logic_state.owned_areas.add(logic_exit)
            for map_exit, req in area.map_exits.items():
                if (
                    map_exit.areakey not in checked_areas
                    and map_exit.areakey not in areas_to_check
                ):
                    if req.is_true(
                        logic_state, logic_state.options, logic_state.macros
                    ):
                        # print(f"now reached {map_exit}")
                        reached_new = True
                        areas_to_check.add(map_exit.areakey)
                        logic_state.owned_areas.add(map_exit.areakey)
        return reached_new

    def _exploration_item_event_subroutine(
        self,
        logic_state: LogicState,
        filled_locations: Dict[Checkname, str],
        done_locations: Set[Checkname],
    ) -> bool:
        collected_new = False
        for areakey in logic_state.owned_areas:
            area = self.areas[areakey]
            for check_name, req in area.locations.items():
                if check_name not in done_locations and req.is_true(
                    logic_state, logic_state.options, logic_state.macros
                ):
                    # print(f"now reached {check_name}")
                    collected_new = True
                    done_locations.add(check_name)
                    item = filled_locations.get(check_name)
                    if item:
                        logic_state.collect_item(item)
            for event_name, req in area.events.items():
                if event_name not in logic_state.owned_events and req.is_true(
                    logic_state, logic_state.options, logic_state.macros
                ):
                    # print(f"now reached {event_name}")
                    collected_new = True
                    logic_state.owned_events.add(event_name)
        return collected_new

    def do_exploration(
        self, logic_state: LogicState, filled_locations: Dict[Checkname, str]
    ):
        last_updated = "locations"
        done_locations: Set[Checkname] = set()
        found_new = True
        while found_new:
            found_new = False
            # first, we collect all possible areas
            if self._exploration_subroutine(logic_state):
                found_new = True
            # collect all now accessible items and events
            if self._exploration_item_event_subroutine(
                logic_state, filled_locations, done_locations
            ):
                found_new = True

    def calculate_spheres(self) -> List[Dict[str, str]]:
        spheres = []
        pass
