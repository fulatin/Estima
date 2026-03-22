#!/bin/bash
# Estima eGUI 启动脚本 (轻量级 egui 版本)

cd "$(dirname "$0")"
./target/release/estima-gui-egui "$@"
