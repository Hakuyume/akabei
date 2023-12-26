#! /usr/bin/env sh

export GTK_IM_MODULE=fcitx
export QT_IM_MODULE=fcitx
export XMODIFIERS=@im=fcitx

export QT_QPA_PLATFORM=wayland

exec sway
