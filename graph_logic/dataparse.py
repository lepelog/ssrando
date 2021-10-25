from dataclasses import dataclass
from enum import Enum
from typing import List, Set, Dict, Optional, Generator, Iterable, Tuple
import yaml
import re
from pathlib import Path

lowest_level_keys = set(
    (
        "locations",
        "events",
        "force-tod",
        "map-exits",
        "logic-exits",
        "can-sleep",
        "macros",
    )
)


@dataclass(frozen=True, eq=True)
class MapExit:
    stage: str
    area: str
    disambiguation: Optional[str]


class ForceTod(Enum):
    Both = "Both"
    Day = "Day"
    Night = "Night"


@dataclass
class Area:
    region: str
    stage: str
    areaname: str
    force_tod: ForceTod
    can_sleep: bool
    locations: Dict[str, "LogicExpression"]
    events: Dict[str, "LogicExpression"]
    map_exits: Dict[MapExit, "LogicExpression"]
    logic_exits: Dict[str, "LogicExpression"]


DISAMBIGUATION = re.compile(r"(.*) \((.*)\)")


def parse_map_exit(map_exit: str) -> MapExit:
    if not " - " in map_exit:
        raise Exception(f"bad map exit: {map_exit}")
    stage, area = map_exit.split(" - ", 1)
    match = DISAMBIGUATION.match(area)
    disambiguation = None
    if match:
        area = match.group(1)
        disambiguation = match.group(2)
    return MapExit(stage, area, disambiguation)


def parse_area(
    regionname: str,
    stagename: str,
    areaname: str,
    areadef,
    force_tod: ForceTod,
    can_sleep: bool,
) -> Area:
    a = areadef.keys() - lowest_level_keys
    assert not a, f"{stagename}-{areaname}{a}"
    return Area(
        regionname,
        stagename,
        areaname,
        areadef.get("force-tod", force_tod),
        areadef.get("can-sleep", can_sleep),
        areadef.get("locations", {}),
        areadef.get("events", {}),
        dict((parse_map_exit(k), v) for k, v in areadef.get("map-exits", {}).items()),
        areadef.get("logic-exits", {}),
    )


def parse_stage(
    regionname: str, stagename: str, stagedef, force_tod: ForceTod
) -> Iterable[Area]:
    a = stagedef.keys() - ("areas", "force-tod", "can-sleep", "stage")
    assert not a, f"{stagename}{a}"
    force_tod = stagedef.get("force-tod", force_tod)
    can_sleep = stagedef.get("can-sleep", False)
    for areaname, areadef in stagedef["areas"].items():
        yield parse_area(regionname, stagename, areaname, areadef, force_tod, can_sleep)


def parse_region(regionname: str, regiondef) -> Iterable[Area]:
    a = regiondef.keys() - ("stages", "force-tod")
    assert not a, a
    force_tod = regiondef.get("force-tod", ForceTod.Both)
    for stagename, stagedef in regiondef["stages"].items():
        yield from parse_stage(regionname, stagename, stagedef, force_tod)


# Endgoal: areamap[area]: AREA^
def parse_yaml(yml) -> Iterable[Area]:
    region = None
    stage = None
    force_tod = "Both"
    can_sleep = False
    for regionname, regiondef in yml.items():
        yield from parse_region(regionname, regiondef)


def get_bad_exits(areas: List[Area]) -> List:
    by_stage_area = {}
    for area in areas:
        by_stage_area[(area.stage, area.areaname)] = area

    # test which exits do not exits:
    for area in areas:
        for logic_exit in area.logic_exits.keys():
            if (area.stage, logic_exit) not in by_stage_area:
                print(f"logic not found!: {area.stage} - {logic_exit}")
        for map_exit in area.map_exits.keys():
            if (map_exit.stage, map_exit.area) not in by_stage_area:
                print(f"map not found!: {map_exit.stage} - {map_exit.area}")


def get_oneway_connections(areas: List[Area]) -> Set[Tuple[str, str]]:
    # write out all one way connections
    # set of (exitstage, entrancestage, disambiguation)
    one_way_connections = set()
    for area in areas:
        for map_exit in area.map_exits.keys():
            connection = (area.stage, map_exit.stage, map_exit.disambiguation)
            rev_connection = (map_exit.stage, area.stage, map_exit.disambiguation)

            if rev_connection in one_way_connections:
                # the current connection already has a fitting counterpart!
                # so it's not 1 way any more
                one_way_connections.remove(rev_connection)
            else:
                # no counterpart, so this connection needs to find its counterpart
                one_way_connections.add(connection)
    return one_way_connections


def parse_all():
    areas = []
    for p in Path("bitless").glob("*.yaml"):
        if "macros" in p.parts[-1]:
            continue
        print(f"parsing {p}")
        with open(p) as f:
            yml = yaml.safe_load(f)
            areas.extend(parse_yaml(yml))

    return areas
