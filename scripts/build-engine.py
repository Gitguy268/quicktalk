"""Build the pinned speech engine; requires Git, CMake, and a C++ compiler."""
from pathlib import Path
import os, platform, shutil, subprocess
root=Path(__file__).resolve().parent.parent
source=root/'.build'/'whisper.cpp'
build=root/'.build'/'whisper-build'
revision='231638a0717b7822784297e5980965ffcec05810'
def run(args): subprocess.run([str(a) for a in args],check=True)
if not source.exists():
    source.parent.mkdir(parents=True,exist_ok=True)
    run(['git','clone','https://github.com/ggml-org/whisper.cpp.git',source])
run(['git','-C',source,'checkout','--detach',revision])
args=['cmake','-S',source,'-B',build,'-DCMAKE_BUILD_TYPE=Release','-DBUILD_SHARED_LIBS=OFF','-DWHISPER_BUILD_TESTS=OFF','-DGGML_NATIVE=OFF','-DGGML_METAL=OFF']
if platform.system()=='Darwin':args+=['-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0']
run(args)
run(['cmake','--build',build,'--config','Release','--parallel','4','--target','whisper-cli'])
suffix='.exe' if os.name=='nt' else ''
binary=next(p for p in [build/'bin'/'Release'/('whisper-cli'+suffix),build/'bin'/('whisper-cli'+suffix)] if p.exists())
dest=root/'src-tauri'/'binaries';dest.mkdir(exist_ok=True)
shutil.copy2(binary,dest/binary.name)
shutil.copy2(root/'scripts'/'speaker-analysis.py',dest/'speaker-analysis.py')
if platform.system()=='Darwin':
    arch='arm64' if platform.machine()=='arm64' else 'x86_64'
    run(['swiftc','-module-cache-path',root/'.build'/'swift-cache','-target',f'{arch}-apple-macos14.0','-parse-as-library','-O',root/'native'/'system-audio.swift','-o',dest/'quicktalk-system-audio','-framework','ScreenCaptureKit','-framework','AVFoundation','-framework','CoreMedia'])
print('Speech engine ready:',dest)
