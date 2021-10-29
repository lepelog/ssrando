import yaml
import random
from collections import defaultdict
from pathlib import Path

from graph_logic.graph_logic import GraphLogic
from graph_logic.logic_types import Checkname
from graph_logic.dataparse import parse_all
from options import Options
from graph_logic.item_types import PROGRESS_ITEMS, DUNGEON_PROGRESS_ITEMS


def to_checkname(s: str) -> Checkname:
    region, check = s.split(" - ", 1)
    return Checkname(region, check)


def main():
    folder = Path("graphtestlogs")
    folder.mkdir(exist_ok=True)
    areas = parse_all()
    with open("checks.yaml") as f:
        checks = yaml.safe_load(f)
    for i in range(200, 300):
        logic = GraphLogic(areas, Options())
        filled_locations = {}
        items_to_place = list(PROGRESS_ITEMS)
        items_to_place.extend(DUNGEON_PROGRESS_ITEMS)
        rng = random.Random(i)
        locations_to_fill = [to_checkname(s) for s in checks.keys()]
        for loc in locations_to_fill:
            if loc.check.startswith("Crystal"):
                filled_locations[loc] = "Gratitude Crystal"
                items_to_place.remove("Gratitude Crystal")
        locations_to_fill = [
            l for l in locations_to_fill if not l.check.startswith("Crystal")
        ]
        logic.fill_assumed(filled_locations, [], items_to_place, locations_to_fill, rng)
        by_region = defaultdict(dict)
        for loc, item in filled_locations.items():
            by_region[loc.region][loc.check] = item
        with (folder / f"log{i:03}.txt").open("w") as f:
            for region, val in by_region.items():
                f.write(f"{region}:\n")
                for checkname, item in val.items():
                    f.write(f"  {checkname:<63}: {item}\n")
        print(f"done with seed {i}")


if __name__ == "__main__":
    main()
