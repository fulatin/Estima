#!/bin/bash
# Estima GUI 启动脚本

# 强制使用 X11 后端
export GDK_BACKEND=x11

# 完全禁用硬件加速
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export LIBGL_ALWAYS_SOFTWARE=1
export GALLIUM_DRIVER=llvmpipe

# WebKit 软件渲染
export WEBKIT_FORCE_SANDBOX=0
export WEBKIT_INSPECTOR_SERVER=9999

# 禁用 GPU
export __GLX_VENDOR_LIBRARY_NAME=mesa
export MESA_GLTHREAD=false

# 运行 GUI
cd "$(dirname "$0")"
./target/release/estima-gui "$@"
