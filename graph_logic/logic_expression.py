from typing import List, Tuple, NewType, DefaultDict, Dict, Callable, Set
from collections import OrderedDict, defaultdict
import re
from dataclasses import dataclass

from .item_types import ALL_ITEM_NAMES
from .logic_types import Areakey
from options import Options


# maybe move to logic_types
class LogicState:
    def __init__(self, options: Options, macros: Dict[str, "LogicExpression"]):
        self.options = options
        self.macros = macros
        self.owned_items: DefaultDict[str, int] = defaultdict(int)
        self.owned_events: Set[str] = set()
        self.owned_areas: Set[Areakey] = set()

    def collect_item(self, item):
        self.owned_items[item] += 1

    def collect_event(self, event):
        self.owned_events.add(event)

    def collect_area(self, area: Areakey):
        self.owned_areas.add(area)

    def has_item(self, item):
        return self.owned_items.get(item, 0) >= 1

    def has_countable_item(self, item, count):
        return self.owned_items.get(item, 0) >= count


LocationName = NewType("LocationName", str)
ItemName = NewType("ItemName", str)


def get_option_check_function(req_name: str) -> Callable[[Options], bool]:
    positive_boolean_match = re.search(r"^Option \"([^\"]+)\" Enabled$", req_name)
    negative_boolean_match = re.search(r"^Option \"([^\"]+)\" Disabled$", req_name)
    positive_dropdown_match = re.search(
        r"^Option \"([^\"]+)\" Is \"([^\"]+)\"$", req_name
    )
    negative_dropdown_match = re.search(
        r"^Option \"([^\"]+)\" Is Not \"([^\"]+)\"$", req_name
    )
    positive_list_match = re.search(
        r"^Option \"([^\"]+)\" Contains \"([^\"]+)\"$", req_name
    )
    negative_list_match = re.search(
        r"^Option \"([^\"]+)\" Does Not Contain \"([^\"]+)\"$", req_name
    )
    if positive_boolean_match:
        option_name = positive_boolean_match.group(1)

        def check(options: Options) -> bool:
            return not not options[option_name]

        return check
    elif negative_boolean_match:
        option_name = negative_boolean_match.group(1)

        def check(options: Options) -> bool:
            return not options[option_name]

        return check
    elif positive_dropdown_match:
        option_name = positive_dropdown_match.group(1)
        value = positive_dropdown_match.group(2)

        def check(options: Options) -> bool:
            return options[option_name] == value

        return check
    elif negative_dropdown_match:
        option_name = negative_dropdown_match.group(1)
        value = negative_dropdown_match.group(2)

        def check(options: Options) -> bool:
            return options[option_name] != value

        return check
    elif positive_list_match:
        option_name = positive_list_match.group(1)
        value = positive_list_match.group(2)

        def check(options: Options) -> bool:
            return value in options[option_name]

        return check
    elif negative_list_match:
        option_name = negative_list_match.group(1)
        value = negative_list_match.group(2)

        def check(options: Options) -> bool:
            return value not in options[option_name]

        return check
    else:
        raise Exception("Invalid option check requirement: %s" % req_name)


def check_option_enabled_requirement(options, req_name):
    positive_boolean_match = re.search(r"^Option \"([^\"]+)\" Enabled$", req_name)
    negative_boolean_match = re.search(r"^Option \"([^\"]+)\" Disabled$", req_name)
    positive_dropdown_match = re.search(
        r"^Option \"([^\"]+)\" Is \"([^\"]+)\"$", req_name
    )
    negative_dropdown_match = re.search(
        r"^Option \"([^\"]+)\" Is Not \"([^\"]+)\"$", req_name
    )
    positive_list_match = re.search(
        r"^Option \"([^\"]+)\" Contains \"([^\"]+)\"$", req_name
    )
    negative_list_match = re.search(
        r"^Option \"([^\"]+)\" Does Not Contain \"([^\"]+)\"$", req_name
    )
    if positive_boolean_match:
        option_name = positive_boolean_match.group(1)
        return not not options.get(option_name)
    elif negative_boolean_match:
        option_name = negative_boolean_match.group(1)
        return not options.get(option_name)
    elif positive_dropdown_match:
        option_name = positive_dropdown_match.group(1)
        value = positive_dropdown_match.group(2)
        return options.get(option_name) == value
    elif negative_dropdown_match:
        option_name = negative_dropdown_match.group(1)
        value = negative_dropdown_match.group(2)
        return options.get(option_name) != value
    elif positive_list_match:
        option_name = positive_list_match.group(1)
        value = positive_list_match.group(2)
        return value in options.get(option_name, [])
    elif negative_list_match:
        option_name = negative_list_match.group(1)
        value = negative_list_match.group(2)
        return value not in options.get(option_name, [])
    else:
        raise Exception("Invalid option check requirement: %s" % req_name)


ITEM_WITH_COUNT_REGEX = re.compile(r"^(.+) x(\d+)$")


class LogicExpression:
    def is_true(
        self,
        logic_state: LogicState,
        options: Options,
        macros: Dict[str, "LogicExpression"],
    ):
        raise NotImplementedError("abstract")

    def specialize(self, macros):
        return self

    def __str__(self):
        raise NotImplementedError("abstract")


# Specialized logic expressions
@dataclass(frozen=True)
class OptionExpression(LogicExpression):
    check: Callable[[Options], bool]

    def is_true(
        self, logic_state, options: Options, macros: Dict[str, LogicExpression]
    ):
        return self.check(options)


@dataclass(frozen=True)
class TrickExpression(LogicExpression):
    name: str

    def is_true(
        self,
        logic_state: LogicState,
        options: Options,
        macros: Dict[str, LogicExpression],
    ):
        if options["logic-mode"] == "BiTless":
            return self.name in options["enabled-tricks-bitless"]
        else:
            return self.name in options["enabled-tricks-glitched"]


@dataclass(frozen=True)
class ItemExpression(LogicExpression):
    item: str
    count: int

    def is_true(
        self,
        logic_state: LogicState,
        options: Options,
        macros: Dict[str, LogicExpression],
    ):
        return logic_state.has_countable_item(self.item, self.count)


@dataclass(frozen=True)
class RawExpression(LogicExpression):
    name: str

    def is_true(
        self,
        logic_state: LogicState,
        options: Options,
        macros: Dict[str, LogicExpression],
    ):
        if self.name in logic_state.owned_events:
            return True
        else:
            expr = macros.get(self.name)
            if expr is not None:
                return expr.is_true(logic_state, options, macros)
            return False


@dataclass(frozen=True)
class ResolvedExpression(LogicExpression):
    value: bool

    def is_true(self, logic_state, options, macros):
        return self.value


@dataclass(frozen=True)
class TimeLogicExpression(LogicExpression):
    time: str

    def is_true(self, logic_state: LogicState, options, macros):
        return self.time == logic_state.time_state


# End special logic expressions


class BaseLogicExpression(LogicExpression):
    def __init__(self, req_name):
        self.req_name = req_name

    def specialize(
        self,
        macros: Dict[str, LogicExpression],
    ) -> LogicExpression:
        match = ITEM_WITH_COUNT_REGEX.match(self.req_name)
        if match:
            item_name = match.group(1)
            num_required = int(match.group(2))

            return ItemExpression(item_name, num_required)
        elif self.req_name.startswith('Option "'):
            return OptionExpression(get_option_check_function(self.req_name))
        elif self.req_name.endswith(" Trick"):
            trickname = self.req_name[: -len(" Trick")]
            return TrickExpression(trickname)
        elif self.req_name in ALL_ITEM_NAMES:
            return ItemExpression(self.req_name, 1)
        elif self.req_name in macros:
            return macros[self.req_name]
        elif self.req_name in ("Nothing", "Nighttime", "Daytime"):
            # treat times as nothing for now
            return ResolvedExpression(True)
        elif self.req_name == "Impossible":
            return ResolvedExpression(False)
        else:
            if self.req_name == "Beetle":
                raise Exception("asdf")
            print(f"hopefully {self.req_name} is an event or macro")
            return RawExpression(self.req_name)

    def is_true(
        self,
        options: Options,
        logic_state: LogicState,
        macros,
    ):
        raise Exception("needs to be specialized")

    def __str__(self):
        return self.req_name


@dataclass
class AndLogicExpression(LogicExpression):
    requirements: List[LogicExpression]

    def is_true(self, options: Options, logic_state: LogicState, macros):
        res = all(
            (req.is_true(options, logic_state, macros) for req in self.requirements)
        )
        return res

    def specialize(self, macros):
        new_reqs = [req.specialize(macros) for req in self.requirements]
        for req in new_reqs:
            assert isinstance(req, LogicExpression), req
        return AndLogicExpression(new_reqs)

    def __str__(self):
        return "(" + (" & ".join((str(req) for req in self.requirements))) + ")"


@dataclass
class OrLogicExpression(LogicExpression):
    requirements: List[LogicExpression]

    def is_true(self, options: Options, logic_state: LogicState, macros):
        res = any(
            (req.is_true(options, logic_state, macros) for req in self.requirements)
        )
        return res

    def specialize(self, macros):
        return OrLogicExpression([req.specialize(macros) for req in self.requirements])

    def __str__(self):
        return "(" + (" | ".join((str(req) for req in self.requirements))) + ")"


def find_closing_parenthesis(tokens: List[str], start: int) -> int:
    assert tokens[start] == "("
    nesting_lvl = 1
    pos = start + 1
    while nesting_lvl > 0 and pos < len(tokens):
        char = tokens[pos]
        if char == "(":
            nesting_lvl += 1
        elif char == ")":
            nesting_lvl -= 1
        pos += 1
    if nesting_lvl != 0:
        raise Exception("parenthesis never closed!")
    return pos - 1


def parse_and_specialize(s: str, macros) -> LogicExpression:
    return parse_logic_expression(s).specialize(macros)


def parse_logic_expression(expression: str) -> LogicExpression:
    tokens = [str.strip() for str in re.split("([&|()])", expression)]
    tokens = [token for token in tokens if token != ""]

    return parse_logic_token_expr(tokens)


def parse_logic_token_expr(tokens: List[str]) -> LogicExpression:
    pos = 0
    logic_type = None  # can be 'or' or 'and'
    parsed = []
    while pos < len(tokens):
        cur_token = tokens[pos]
        if cur_token == "(":
            end = find_closing_parenthesis(tokens, pos)
            parsed.append(parse_logic_token_expr(tokens[pos + 1 : end]))
            pos = end + 1
        elif cur_token == "&":
            if logic_type == "or":
                raise Exception("mixed '&' and '|'!")
            else:
                logic_type = "and"
            pos += 1
        elif cur_token == "|":
            if logic_type == "and":
                raise Exception("mixed '&' and '|'!")
            else:
                logic_type = "or"
            pos += 1
        else:
            parsed.append(BaseLogicExpression(cur_token))
            pos += 1
    if logic_type == None:
        assert len(parsed) == 1
        return parsed[0]
    elif logic_type == "and":
        return AndLogicExpression(parsed)
    elif logic_type == "or":
        return OrLogicExpression(parsed)
    else:
        raise Exception(logic_type)


def test():
    import yaml

    with open("SS Rando Logic - Item Location.yaml") as f:
        locations = yaml.safe_load(f)
    for loc in locations:
        req_str = locations[loc]["Need"]
        print()
        print(req_str)
        print(str(parse_logic_expression(req_str)))
