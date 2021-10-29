from dataclasses import dataclass
from typing import Optional, Dict
from enum import Enum


@dataclass(frozen=True, eq=True)
class Areakey:
    """
    A unique key to identify an area
    area names occur multiple times (e.g. "Main"),
    but they are unique within a stage
    """

    stage: str
    area: str


@dataclass(frozen=True, eq=True)
class Checkname:
    region: str
    check: str


@dataclass(frozen=True, eq=True)
class Passageway:
    """
    Represents both entrances and exits"""

    areakey: Areakey
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
    fullareaname: str
    force_tod: ForceTod
    can_sleep: bool
    locations: Dict[Checkname, "LogicExpression"]
    events: Dict[str, "LogicExpression"]
    map_exits: Dict[Passageway, "LogicExpression"]
    logic_exits: Dict[Areakey, "LogicExpression"]

    def get_areakey(self) -> Areakey:
        return Areakey(self.stage, self.areaname)
