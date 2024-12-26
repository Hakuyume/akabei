#
# ~/.bashrc
#

# If not running interactively, don't do anything
[[ $- != *i* ]] && return

alias ls='ls --color=auto'
alias grep='grep --color=auto'
PS1='[\u@\h \W]\$ '

for P in ${HOME}/.cargo/bin ${HOME}/.local/bin
do
    [[ ":$PATH:" = *":$P:"* ]] || export PATH="$P:$PATH"
done
unset P

export MOZ_DBUS_REMOTE=1
export SSH_AUTH_SOCK=${XDG_RUNTIME_DIR}/ssh-agent.socket

eval "$(starship init bash)"
