#
# ~/.bashrc
#

# If not running interactively, don't do anything
[[ $- != *i* ]] && return

alias ls='ls --color=auto'
alias grep='grep --color=auto'
PS1='[\u@\h \W]\$ '

function __ensure_path {
    [[ ":${PATH}:" = *":$1:"* ]] || export PATH="$1:${PATH}"
}
__ensure_path ${HOME}/.local/bin

for f in ${HOME}/.bashrc.d/*.bash
do
    . $f
done
unset f
