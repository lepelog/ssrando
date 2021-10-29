from paths import RANDO_ROOT_PATH
import yaml

CACHE = {}


def read_yaml_file_cached(filename: str):
    if filename in CACHE:
        return CACHE[filename]
    else:
        with (RANDO_ROOT_PATH / filename).open() as f:
            yaml_file = yaml.safe_load(f)

        # some files need special handling
        if filename == "checks.yaml":
            for location_name in yaml_file:
                if not "type" in yaml_file[location_name]:
                    print("ERROR, " + location_name + " doesn't have types!")
                types_string = yaml_file[location_name]["type"]
                types = types_string.split(",")
                types = set((type.strip() for type in types))
                unknown_types = [x for x in types if not x in constants.ALL_TYPES]
                if len(unknown_types) != 0:
                    raise Exception(f"unknown types: {unknown_types}")
                yaml_file[location_name]["type"] = types
        CACHE[filename] = yaml_file
        return yaml_file


class YamlOrderedDictLoader(yaml.SafeLoader):
    pass


YamlOrderedDictLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
    lambda loader, node: OrderedDict(loader.construct_pairs(node)),
)
