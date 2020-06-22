from context import sslib
from util import get_bzs_data, get_arc_data, ALL_STAGES
import pytest
from io import BytesIO
import nlzss11

@pytest.mark.parametrize("stage", ALL_STAGES)
# @pytest.mark.parametrize("stage", ['F000'])
def test_roundtrip(stage):
    with open(f'../actual-extract/DATA/files/Stage/{stage}/{stage}_stg_l0.arc.LZ','rb') as f:
        extracted_data = nlzss11.decompress(f.read())
    stagearc = sslib.U8File.parse_u8(BytesIO(extracted_data))
    data = stagearc.get_file_data('dat/stage.bzs')
    parsed = sslib.parseBzs(data)
    roomcount = len(parsed.get('RMPL',[]))
    assert data == sslib.buildBzs(parsed)
    for i in range(roomcount):
        roomdata = stagearc.get_file_data(f'rarc/{stage}_r{i:02}.arc')
        if not roomdata:
            continue
        roomdata = sslib.U8File.parse_u8(BytesIO(roomdata)).get_file_data('dat/room.bzs')
        assert roomdata == sslib.buildBzs(sslib.parseBzs(roomdata))
