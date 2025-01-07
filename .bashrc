#
# ~/.bashrc
#

# If not running interactively, don't do anything
[[ $- != *i* ]] && return

alias ls='ls --color=auto'
alias grep='grep --color=auto'
PS1='[\u@\h \W]\$ '

for P in ${HOME}/.cargo/bin ${HOME}/.krew/bin ${HOME}/.local/bin
do
    if [[ ! ":${PATH}:" = *":${P}:"* ]] && [ -d ${P} ]; then
        export PATH="${P}:${PATH}"
    fi
done
unset P

export SSH_AUTH_SOCK=${XDG_RUNTIME_DIR}/ssh-agent.socket

if which firefox >/dev/null 2>&1; then
    export MOZ_DBUS_REMOTE=1
fi

if which podman >/dev/null 2>&1; then
    export DOCKER_HOST="unix://$(podman info --format='{{.Host.RemoteSocket.Path}}')"
    if [[ ! -v GOOGLE_APPLICATION_CREDENTIALS ]] && podman volume exists systemd-google-cloud-cli-config; then
        export GOOGLE_APPLICATION_CREDENTIALS="$(podman volume inspect --format='{{.Mountpoint}}' systemd-google-cloud-cli-config)/gcloud/application_default_credentials.json"
    fi
    export KIND_EXPERIMENTAL_PROVIDER=podman
fi

eval "$(starship init bash)"
