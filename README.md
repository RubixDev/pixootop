# PixooTop

Simple system monitor on the
[Divoom Pixoo](https://divoom.com/products/divoom-pixoo/).

Displays info about volume, RAM usage, CPU usage, GPU usage, GPU VRAM usage,
network upload, and network download in that order as colored progress bars, as
well as the current time up to the second.

## Usage

Currently, this program is very specific to my exact PC setup. It only works on
Linux, only with AMD GPUs, and several device names etc. are hard-coded as
constants at the top.

The project consists of a server and a client binary. The server is meant to run
on some always-on device that has bluetooth access to the Pixoo, such as a
Raspberry Pi. The client binary can then run on your PC which will continously
send the system usage information to the server. This split allows the clock to
keep being displayed while your PC is off.
