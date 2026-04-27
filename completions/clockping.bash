_clockping() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="clockping"
                ;;
            clockping,completion)
                cmd="clockping__subcmd__completion"
                ;;
            clockping,gtp)
                cmd="clockping__subcmd__gtp"
                ;;
            clockping,help)
                cmd="clockping__subcmd__help"
                ;;
            clockping,http)
                cmd="clockping__subcmd__http"
                ;;
            clockping,icmp)
                cmd="clockping__subcmd__icmp"
                ;;
            clockping,tcp)
                cmd="clockping__subcmd__tcp"
                ;;
            clockping__subcmd__gtp,help)
                cmd="clockping__subcmd__gtp__subcmd__help"
                ;;
            clockping__subcmd__gtp,v1c)
                cmd="clockping__subcmd__gtp__subcmd__v1c"
                ;;
            clockping__subcmd__gtp,v1u)
                cmd="clockping__subcmd__gtp__subcmd__v1u"
                ;;
            clockping__subcmd__gtp,v2c)
                cmd="clockping__subcmd__gtp__subcmd__v2c"
                ;;
            clockping__subcmd__gtp__subcmd__help,help)
                cmd="clockping__subcmd__gtp__subcmd__help__subcmd__help"
                ;;
            clockping__subcmd__gtp__subcmd__help,v1c)
                cmd="clockping__subcmd__gtp__subcmd__help__subcmd__v1c"
                ;;
            clockping__subcmd__gtp__subcmd__help,v1u)
                cmd="clockping__subcmd__gtp__subcmd__help__subcmd__v1u"
                ;;
            clockping__subcmd__gtp__subcmd__help,v2c)
                cmd="clockping__subcmd__gtp__subcmd__help__subcmd__v2c"
                ;;
            clockping__subcmd__help,completion)
                cmd="clockping__subcmd__help__subcmd__completion"
                ;;
            clockping__subcmd__help,gtp)
                cmd="clockping__subcmd__help__subcmd__gtp"
                ;;
            clockping__subcmd__help,help)
                cmd="clockping__subcmd__help__subcmd__help"
                ;;
            clockping__subcmd__help,http)
                cmd="clockping__subcmd__help__subcmd__http"
                ;;
            clockping__subcmd__help,icmp)
                cmd="clockping__subcmd__help__subcmd__icmp"
                ;;
            clockping__subcmd__help,tcp)
                cmd="clockping__subcmd__help__subcmd__tcp"
                ;;
            clockping__subcmd__help__subcmd__gtp,v1c)
                cmd="clockping__subcmd__help__subcmd__gtp__subcmd__v1c"
                ;;
            clockping__subcmd__help__subcmd__gtp,v1u)
                cmd="clockping__subcmd__help__subcmd__gtp__subcmd__v1u"
                ;;
            clockping__subcmd__help__subcmd__gtp,v2c)
                cmd="clockping__subcmd__help__subcmd__gtp__subcmd__v2c"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        clockping)
            opts="-h -V --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version icmp tcp http gtp completion help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__completion)
            opts="-h -V --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version bash elvish fish powershell zsh"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp)
            opts="-h -V --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version v1u v1c v2c help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp__subcmd__help)
            opts="v1u v1c v2c help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp__subcmd__help__subcmd__v1c)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp__subcmd__help__subcmd__v1u)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp__subcmd__help__subcmd__v2c)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp__subcmd__v1c)
            opts="-c -i -W -w -q -h -V --count --interval --timeout --deadline --port --quiet --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version <TARGET>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -W)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --deadline)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --port)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp__subcmd__v1u)
            opts="-c -i -W -w -q -h -V --count --interval --timeout --deadline --port --quiet --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version <TARGET>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -W)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --deadline)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --port)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__gtp__subcmd__v2c)
            opts="-c -i -W -w -q -h -V --count --interval --timeout --deadline --port --quiet --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version <TARGET>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -W)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --deadline)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --port)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help)
            opts="icmp tcp http gtp completion help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__completion)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__gtp)
            opts="v1u v1c v2c"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__gtp__subcmd__v1c)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__gtp__subcmd__v1u)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__gtp__subcmd__v2c)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__http)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__icmp)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__help__subcmd__tcp)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__http)
            opts="-4 -6 -c -i -W -w -X -H -L -k -q -h -V --count --interval --timeout --deadline --method --ok-status --header --location --insecure --quiet --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version <TARGET>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -W)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --deadline)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --method)
                    COMPREPLY=($(compgen -W "head get" -- "${cur}"))
                    return 0
                    ;;
                -X)
                    COMPREPLY=($(compgen -W "head get" -- "${cur}"))
                    return 0
                    ;;
                --ok-status)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --header)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -H)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__icmp)
            opts="-h -V --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version [ARGS]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        clockping__subcmd__tcp)
            opts="-4 -6 -c -i -W -w -q -h -V --count --interval --timeout --deadline --quiet --ts.preset --ts.format --out.format --out.colored --push.url --push.delete-on-exit --push.interval --push.job --push.label --push.retries --push.timeout --push.user-agent --metrics.file --metrics.format --metrics.label --metrics.prefix --help --version <TARGET>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -W)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --deadline)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ts.preset)
                    COMPREPLY=($(compgen -W "local rfc3339 unix unix-ms none" -- "${cur}"))
                    return 0
                    ;;
                --ts.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --out.format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --push.url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.job)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.retries)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --push.user-agent)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --metrics.prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _clockping -o nosort -o bashdefault -o default clockping
else
    complete -F _clockping -o bashdefault -o default clockping
fi
