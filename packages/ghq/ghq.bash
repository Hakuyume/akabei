function ghq-cd {
    local query
    query="$(ghq list | sk --color=molokai)" && cd "$(ghq list --exact --full-path "${query}")"
}
