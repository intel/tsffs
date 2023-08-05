# AVD: Auto Video Documentation

Automatically generate video docs from specifications.

## Dependencies

You need:

- cmake
- rust
- python3
- git
- Ninja (recommended)
- bark (See below)
- OBS (See below)
- Kitty terminal

You also need an NVidia GPU.

## Install bark

```
python3 -m pip install --upgrade pip setuptools
pip install git+https://github.com/suno-ai/bark.git
python3 -m bark -h # This should work
```

## Install OBS

Install OBS from their site. Then, go to Tools -> Web Socket Server Settings -> Enable
WebSocket Server.

## Create custom model

Use serp-ai/bark-with-voice-clone/clone_voice.ipynb with `arecord -f FLOAT_LE -r 24000 -d 12 ~/hub/bark-with-voice-clone/audio.wav`
to create a sample yourname.npz

Then copy it here