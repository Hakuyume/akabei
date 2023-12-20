#! /usr/bin/env sh

export GTK_IM_MODULE=fcitx
export QT_IM_MODULE=fcitx
export XMODIFIERS=@im=fcitx

export QT_QPA_PLATFORM=wayland

export MOZ_DBUS_REMOTE=1
export MOZ_ENABLE_WAYLAND=1

exec sway
