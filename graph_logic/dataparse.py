from dataclasses import dataclass
from enum import Enum, Flag
from typing import List, Set, Dict, Optional, Generator, Iterable, Tuple
import yaml
import re
from pathlib import Path
from .logic_expression import (
    LogicExpression,
    OrLogicExpression,
    AndLogicExpression,
    RawExpression,
    parse_logic_expression,
    parse_and_specialize,
)
from .item_types import ALL_ITEM_NAMES
from .logic_types import Area, Areakey, Passageway, ForceTod, Checkname

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


DISAMBIGUATION = re.compile(r"(.*) \((.*)\)")


def parse_map_exit(map_exit: str) -> Passageway:
    if not " - " in map_exit:
        raise Exception(f"bad map exit: {map_exit}")
    stage, area = map_exit.split(" - ", 1)
    match = DISAMBIGUATION.match(area)
    disambiguation = None
    if match:
        area = match.group(1)
        disambiguation = match.group(2)
    return Passageway(Areakey(stage, area), disambiguation)


def parse_area(
    regionname: str,
    stagename: str,
    areaname: str,
    areadef,
    force_tod: ForceTod,
    can_sleep: bool,
    macros,
) -> Area:
    a = areadef.keys() - lowest_level_keys
    assert not a, f"{stagename}-{areaname}{a}"
    local_macros = dict(
        (k, parse_and_specialize(v, macros))
        for (k, v) in areadef.get("macros", {}).items()
    )
    local_macros.update(macros)
    return Area(
        regionname,
        stagename,
        areaname,
        f"{stagename} - {areaname}",
        ForceTod(areadef.get("force-tod", force_tod)),
        areadef.get("can-sleep", can_sleep),
        dict(
            (Checkname(regionname, k), parse_and_specialize(v, local_macros))
            for (k, v) in areadef.get("locations", {}).items()
        ),
        dict(
            (k, parse_and_specialize(v, local_macros))
            for (k, v) in areadef.get("events", {}).items()
        ),
        dict(
            (parse_map_exit(k), parse_and_specialize(v, local_macros))
            for k, v in areadef.get("map-exits", {}).items()
        ),
        dict(
            (Areakey(stagename, k), parse_and_specialize(v, local_macros))
            for (k, v) in areadef.get("logic-exits", {}).items()
        ),
    )


def parse_stage(
    regionname: str, stagename: str, stagedef, force_tod: ForceTod, macros
) -> Iterable[Area]:
    a = stagedef.keys() - ("areas", "force-tod", "can-sleep", "stage")
    assert not a, f"{stagename}{a}"
    force_tod = stagedef.get("force-tod", force_tod)
    can_sleep = stagedef.get("can-sleep", False)
    for areaname, areadef in stagedef["areas"].items():
        yield parse_area(
            regionname, stagename, areaname, areadef, force_tod, can_sleep, macros
        )


def parse_region(regionname: str, regiondef, macros) -> Iterable[Area]:
    a = regiondef.keys() - ("stages", "force-tod")
    assert not a, a
    force_tod = regiondef.get("force-tod", ForceTod.Both)
    for stagename, stagedef in regiondef["stages"].items():
        yield from parse_stage(regionname, stagename, stagedef, force_tod, macros)


# Endgoal: areamap[area]: AREA^
def parse_yaml(yml, macros) -> Iterable[Area]:
    region = None
    stage = None
    force_tod = "Both"
    can_sleep = False
    for regionname, regiondef in yml.items():
        yield from parse_region(regionname, regiondef, macros)


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


def get_event_names(areas: List[Area]) -> Set[str]:
    events = set()
    for area in areas:
        events.update(area.events.keys())
    return events


def _get_raw_req(expr: LogicExpression, s: Set[str]):
    if isinstance(expr, OrLogicExpression) or isinstance(expr, AndLogicExpression):
        for req in expr.requirements:
            _get_raw_req(req, s)
    elif isinstance(expr, RawExpression):
        s.add(expr.name)


def get_raw_requirements(areas: List[Area]) -> Set[str]:
    reqs = set()
    for area in areas:
        for loc in area.locations.values():
            _get_raw_req(loc, reqs)
        for loc in area.events.values():
            _get_raw_req(loc, reqs)
        for loc in area.logic_exits.values():
            _get_raw_req(loc, reqs)
        for loc in area.map_exits.values():
            _get_raw_req(loc, reqs)
    return reqs


def parse_all():
    areas = []
    path = Path(__file__).parent
    with open(path / "bitless" / "macros.yaml") as f:
        macros = yaml.safe_load(f)
        parsed_macros = {}
        for name, macro in macros.items():
            parsed_macros[name] = parse_logic_expression(macro).specialize(
                parsed_macros
            )
    for p in (path / "bitless").glob("*.yaml"):
        if "macros" in p.parts[-1]:
            continue
        print(f"parsing {p}")
        with open(p) as f:
            yml = yaml.safe_load(f)
            areas.extend(parse_yaml(yml, parsed_macros))

    return areas
