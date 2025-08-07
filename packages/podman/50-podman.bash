export DOCKER_HOST="unix://$(podman info --format='{{.Host.RemoteSocket.Path}}')"
export KIND_EXPERIMENTAL_PROVIDER=podman
