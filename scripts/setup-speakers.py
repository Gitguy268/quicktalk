"""Run explicitly to install the optional, memory-intensive speaker worker."""
import pathlib, subprocess, sys, venv
root = pathlib.Path(__file__).resolve().parent.parent
folder = root / '.speaker-venv'
venv.EnvBuilder(with_pip=True).create(folder)
python = folder / ('Scripts/python.exe' if sys.platform == 'win32' else 'bin/python')
subprocess.run([str(python), '-m', 'pip', 'install', 'pyannote.audio>=4,<5', 'soundfile>=0.13,<1'], check=True)
print('Speaker Python path:', python)
print('Accept the speaker-diarization-community-1 model access terms on Hugging Face, then add your read token in QuickTalk.')
