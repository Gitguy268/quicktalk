"""Optional local speaker clustering. Install with scripts/setup-speakers.py first."""
import json, os, sys
from pathlib import Path
import numpy as np
import soundfile as sf
import torch
from pyannote.audio import Pipeline

def main():
    request = json.loads(sys.stdin.read())
    waveform, rate = sf.read(request['audio'], dtype='float32', always_2d=True)
    # Anonymous voice clusters only. Names are supplied by the user in QuickTalk.
    pipeline = Pipeline.from_pretrained('pyannote/speaker-diarization-community-1', token=os.environ.get('HF_TOKEN'))
    result = pipeline({'waveform': torch.from_numpy(waveform.T.copy()), 'sample_rate': rate})
    annotation = getattr(result, 'speaker_diarization', result)
    turns = [{'start': float(turn.start), 'end': float(turn.end), 'speaker': speaker}
             for turn, _, speaker in annotation.itertracks(yield_label=True)]
    Path(request['output']).write_text(json.dumps(turns), encoding='utf-8')

if __name__ == '__main__':
    main()
