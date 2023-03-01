import sys
import re

if len(sys.argv) != 2:
    print(f"usage: {sys.argv[0]} file.h")
    exit(1)

unk_member_re = re.compile(r"\s+undefined field[0-9]+_0x([0-9a-z]+);.*")

with open(sys.argv[1]) as f:
    current_unk_group_start = None
    current_unk_group_last = 0
    for line in f.readlines():
        if (match := unk_member_re.match(line)) is not None:
            addr = int(match.group(1), base=16)
            current_unk_group_last = addr
            if current_unk_group_start is None:
                current_unk_group_start = addr
        else:
            if current_unk_group_start is not None:
                print(
                    f"    undefined field_0x{current_unk_group_start:x}[0x{current_unk_group_last+1:x} - 0x{current_unk_group_start:x}];"
                )
                current_unk_group_start = None
            print(line, end="")
