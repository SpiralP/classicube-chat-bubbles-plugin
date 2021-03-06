# Chat Bubbles plugin for [ClassiCube](https://www.classicube.net/)

## Installing

- Download the latest plugin from GitHub [Releases](https://github.com/SpiralP/classicube-chat-bubbles-plugin/releases/latest)
  - You can find your version by running `/client gpuinfo` ingame.
  - [classicube_chat_bubbles_windows_x86_64.dll](https://github.com/SpiralP/classicube-chat-bubbles-plugin/releases/latest/download/classicube_chat_bubbles_windows_x86_64.dll) for Windows 64 bit ClassiCube
  - [classicube_chat_bubbles_windows_i686.dll](https://github.com/SpiralP/classicube-chat-bubbles-plugin/releases/latest/download/classicube_chat_bubbles_windows_i686.dll) for Windows 32 bit ClassiCube
  - [classicube_chat_bubbles_linux_x86_64.so](https://github.com/SpiralP/classicube-chat-bubbles-plugin/releases/latest/download/classicube_chat_bubbles_linux_x86_64.so) for Linux 64 bit ClassiCube
  - ~~[classicube_chat_bubbles_linux_i686.so](https://github.com/SpiralP/classicube-chat-bubbles-plugin/releases/latest/download/classicube_chat_bubbles_linux_i686.so) for Linux 32 bit ClassiCube~~
  - ~~[classicube_chat_bubbles_linux_armhf.so](https://github.com/SpiralP/classicube-chat-bubbles-plugin/releases/latest/download/classicube_chat_bubbles_linux_armhf.so) for Raspberry pi (Linux armhf/ARMv7) 32 bit ClassiCube~~
  - ~~[classicube_chat_bubbles_linux_aarch64.so](https://github.com/SpiralP/classicube-chat-bubbles-plugin/releases/latest/download/classicube_chat_bubbles_linux_aarch64.so) for Linux aarch64/ARMv8 64 bit ClassiCube~~
  - [classicube_chat_bubbles_macos_x86_64.dylib](https://github.com/SpiralP/classicube-chat-bubbles-plugin/releases/latest/download/classicube_chat_bubbles_macos_x86_64.dylib) for macOS 64 bit ClassiCube
- Put the dll into the `plugins` folder where `ClassiCube.exe` lives

## Troubleshooting

- `The specified module could not be found. (126)`
  - Make sure your ClassiCube is named "ClassiCube.exe" exactly and not something like "ClassiCube (1).exe"
- `The specified procedure could not be found. (127)`
- `The procedure entry point ... could not be located in the dynamic link library`
  - Try updating your ClassiCube from the launcher
- `assertion failed: cell.borrow().is_none()`
  - Multiple plugin dll's are in the plugins folder, remove the duplicate.
- `no suitable image found`
  - You might be using the 32 bit ClassiCube app when you need to be using the 64 bit one
- `[0607/183530.063:FATAL:tsf_text_store.cc(52)] Failed to initialize CategoryMgr.`
  - Using outdated chatsounds plugin
- `A dynamic link library (DLL) initialization routine failed. (os error 1114)`
  - maybe another dll like ReShade is causing weirdness
