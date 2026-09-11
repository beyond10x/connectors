_connectors() {
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
                cmd="connectors"
                ;;
            connectors,adapters)
                cmd="connectors__subcmd__adapters"
                ;;
            connectors,approvals)
                cmd="connectors__subcmd__approvals"
                ;;
            connectors,completions)
                cmd="connectors__subcmd__completions"
                ;;
            connectors,connections)
                cmd="connectors__subcmd__connections"
                ;;
            connectors,help)
                cmd="connectors__subcmd__help"
                ;;
            connectors,operations)
                cmd="connectors__subcmd__operations"
                ;;
            connectors,setup)
                cmd="connectors__subcmd__setup"
                ;;
            connectors__subcmd__adapters,describe)
                cmd="connectors__subcmd__adapters__subcmd__describe"
                ;;
            connectors__subcmd__adapters,help)
                cmd="connectors__subcmd__adapters__subcmd__help"
                ;;
            connectors__subcmd__adapters,list)
                cmd="connectors__subcmd__adapters__subcmd__list"
                ;;
            connectors__subcmd__adapters,status)
                cmd="connectors__subcmd__adapters__subcmd__status"
                ;;
            connectors__subcmd__adapters,stop)
                cmd="connectors__subcmd__adapters__subcmd__stop"
                ;;
            connectors__subcmd__adapters__subcmd__help,describe)
                cmd="connectors__subcmd__adapters__subcmd__help__subcmd__describe"
                ;;
            connectors__subcmd__adapters__subcmd__help,help)
                cmd="connectors__subcmd__adapters__subcmd__help__subcmd__help"
                ;;
            connectors__subcmd__adapters__subcmd__help,list)
                cmd="connectors__subcmd__adapters__subcmd__help__subcmd__list"
                ;;
            connectors__subcmd__adapters__subcmd__help,status)
                cmd="connectors__subcmd__adapters__subcmd__help__subcmd__status"
                ;;
            connectors__subcmd__adapters__subcmd__help,stop)
                cmd="connectors__subcmd__adapters__subcmd__help__subcmd__stop"
                ;;
            connectors__subcmd__approvals,clock-check)
                cmd="connectors__subcmd__approvals__subcmd__clock__subcmd__check"
                ;;
            connectors__subcmd__approvals,help)
                cmd="connectors__subcmd__approvals__subcmd__help"
                ;;
            connectors__subcmd__approvals,issue)
                cmd="connectors__subcmd__approvals__subcmd__issue"
                ;;
            connectors__subcmd__approvals,key-init)
                cmd="connectors__subcmd__approvals__subcmd__key__subcmd__init"
                ;;
            connectors__subcmd__approvals,key-recover)
                cmd="connectors__subcmd__approvals__subcmd__key__subcmd__recover"
                ;;
            connectors__subcmd__approvals,key-retire)
                cmd="connectors__subcmd__approvals__subcmd__key__subcmd__retire"
                ;;
            connectors__subcmd__approvals,key-revoke)
                cmd="connectors__subcmd__approvals__subcmd__key__subcmd__revoke"
                ;;
            connectors__subcmd__approvals,key-rotate)
                cmd="connectors__subcmd__approvals__subcmd__key__subcmd__rotate"
                ;;
            connectors__subcmd__approvals,key-status)
                cmd="connectors__subcmd__approvals__subcmd__key__subcmd__status"
                ;;
            connectors__subcmd__approvals,policy-set)
                cmd="connectors__subcmd__approvals__subcmd__policy__subcmd__set"
                ;;
            connectors__subcmd__approvals,policy-status)
                cmd="connectors__subcmd__approvals__subcmd__policy__subcmd__status"
                ;;
            connectors__subcmd__approvals,prepare)
                cmd="connectors__subcmd__approvals__subcmd__prepare"
                ;;
            connectors__subcmd__approvals__subcmd__help,clock-check)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__clock__subcmd__check"
                ;;
            connectors__subcmd__approvals__subcmd__help,help)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__help"
                ;;
            connectors__subcmd__approvals__subcmd__help,issue)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__issue"
                ;;
            connectors__subcmd__approvals__subcmd__help,key-init)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__init"
                ;;
            connectors__subcmd__approvals__subcmd__help,key-recover)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__recover"
                ;;
            connectors__subcmd__approvals__subcmd__help,key-retire)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__retire"
                ;;
            connectors__subcmd__approvals__subcmd__help,key-revoke)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__revoke"
                ;;
            connectors__subcmd__approvals__subcmd__help,key-rotate)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__rotate"
                ;;
            connectors__subcmd__approvals__subcmd__help,key-status)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__status"
                ;;
            connectors__subcmd__approvals__subcmd__help,policy-set)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__policy__subcmd__set"
                ;;
            connectors__subcmd__approvals__subcmd__help,policy-status)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__policy__subcmd__status"
                ;;
            connectors__subcmd__approvals__subcmd__help,prepare)
                cmd="connectors__subcmd__approvals__subcmd__help__subcmd__prepare"
                ;;
            connectors__subcmd__connections,connect)
                cmd="connectors__subcmd__connections__subcmd__connect"
                ;;
            connectors__subcmd__connections,describe)
                cmd="connectors__subcmd__connections__subcmd__describe"
                ;;
            connectors__subcmd__connections,help)
                cmd="connectors__subcmd__connections__subcmd__help"
                ;;
            connectors__subcmd__connections,list)
                cmd="connectors__subcmd__connections__subcmd__list"
                ;;
            connectors__subcmd__connections,repair)
                cmd="connectors__subcmd__connections__subcmd__repair"
                ;;
            connectors__subcmd__connections,revalidate)
                cmd="connectors__subcmd__connections__subcmd__revalidate"
                ;;
            connectors__subcmd__connections,revoke)
                cmd="connectors__subcmd__connections__subcmd__revoke"
                ;;
            connectors__subcmd__connections,status)
                cmd="connectors__subcmd__connections__subcmd__status"
                ;;
            connectors__subcmd__connections__subcmd__help,connect)
                cmd="connectors__subcmd__connections__subcmd__help__subcmd__connect"
                ;;
            connectors__subcmd__connections__subcmd__help,describe)
                cmd="connectors__subcmd__connections__subcmd__help__subcmd__describe"
                ;;
            connectors__subcmd__connections__subcmd__help,help)
                cmd="connectors__subcmd__connections__subcmd__help__subcmd__help"
                ;;
            connectors__subcmd__connections__subcmd__help,list)
                cmd="connectors__subcmd__connections__subcmd__help__subcmd__list"
                ;;
            connectors__subcmd__connections__subcmd__help,repair)
                cmd="connectors__subcmd__connections__subcmd__help__subcmd__repair"
                ;;
            connectors__subcmd__connections__subcmd__help,revalidate)
                cmd="connectors__subcmd__connections__subcmd__help__subcmd__revalidate"
                ;;
            connectors__subcmd__connections__subcmd__help,revoke)
                cmd="connectors__subcmd__connections__subcmd__help__subcmd__revoke"
                ;;
            connectors__subcmd__connections__subcmd__help,status)
                cmd="connectors__subcmd__connections__subcmd__help__subcmd__status"
                ;;
            connectors__subcmd__help,adapters)
                cmd="connectors__subcmd__help__subcmd__adapters"
                ;;
            connectors__subcmd__help,approvals)
                cmd="connectors__subcmd__help__subcmd__approvals"
                ;;
            connectors__subcmd__help,completions)
                cmd="connectors__subcmd__help__subcmd__completions"
                ;;
            connectors__subcmd__help,connections)
                cmd="connectors__subcmd__help__subcmd__connections"
                ;;
            connectors__subcmd__help,help)
                cmd="connectors__subcmd__help__subcmd__help"
                ;;
            connectors__subcmd__help,operations)
                cmd="connectors__subcmd__help__subcmd__operations"
                ;;
            connectors__subcmd__help,setup)
                cmd="connectors__subcmd__help__subcmd__setup"
                ;;
            connectors__subcmd__help__subcmd__adapters,describe)
                cmd="connectors__subcmd__help__subcmd__adapters__subcmd__describe"
                ;;
            connectors__subcmd__help__subcmd__adapters,list)
                cmd="connectors__subcmd__help__subcmd__adapters__subcmd__list"
                ;;
            connectors__subcmd__help__subcmd__adapters,status)
                cmd="connectors__subcmd__help__subcmd__adapters__subcmd__status"
                ;;
            connectors__subcmd__help__subcmd__adapters,stop)
                cmd="connectors__subcmd__help__subcmd__adapters__subcmd__stop"
                ;;
            connectors__subcmd__help__subcmd__approvals,clock-check)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__clock__subcmd__check"
                ;;
            connectors__subcmd__help__subcmd__approvals,issue)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__issue"
                ;;
            connectors__subcmd__help__subcmd__approvals,key-init)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__init"
                ;;
            connectors__subcmd__help__subcmd__approvals,key-recover)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__recover"
                ;;
            connectors__subcmd__help__subcmd__approvals,key-retire)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__retire"
                ;;
            connectors__subcmd__help__subcmd__approvals,key-revoke)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__revoke"
                ;;
            connectors__subcmd__help__subcmd__approvals,key-rotate)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__rotate"
                ;;
            connectors__subcmd__help__subcmd__approvals,key-status)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__status"
                ;;
            connectors__subcmd__help__subcmd__approvals,policy-set)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__policy__subcmd__set"
                ;;
            connectors__subcmd__help__subcmd__approvals,policy-status)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__policy__subcmd__status"
                ;;
            connectors__subcmd__help__subcmd__approvals,prepare)
                cmd="connectors__subcmd__help__subcmd__approvals__subcmd__prepare"
                ;;
            connectors__subcmd__help__subcmd__connections,connect)
                cmd="connectors__subcmd__help__subcmd__connections__subcmd__connect"
                ;;
            connectors__subcmd__help__subcmd__connections,describe)
                cmd="connectors__subcmd__help__subcmd__connections__subcmd__describe"
                ;;
            connectors__subcmd__help__subcmd__connections,list)
                cmd="connectors__subcmd__help__subcmd__connections__subcmd__list"
                ;;
            connectors__subcmd__help__subcmd__connections,repair)
                cmd="connectors__subcmd__help__subcmd__connections__subcmd__repair"
                ;;
            connectors__subcmd__help__subcmd__connections,revalidate)
                cmd="connectors__subcmd__help__subcmd__connections__subcmd__revalidate"
                ;;
            connectors__subcmd__help__subcmd__connections,revoke)
                cmd="connectors__subcmd__help__subcmd__connections__subcmd__revoke"
                ;;
            connectors__subcmd__help__subcmd__connections,status)
                cmd="connectors__subcmd__help__subcmd__connections__subcmd__status"
                ;;
            connectors__subcmd__help__subcmd__operations,describe)
                cmd="connectors__subcmd__help__subcmd__operations__subcmd__describe"
                ;;
            connectors__subcmd__help__subcmd__operations,invoke)
                cmd="connectors__subcmd__help__subcmd__operations__subcmd__invoke"
                ;;
            connectors__subcmd__help__subcmd__operations,list)
                cmd="connectors__subcmd__help__subcmd__operations__subcmd__list"
                ;;
            connectors__subcmd__help__subcmd__setup,check)
                cmd="connectors__subcmd__help__subcmd__setup__subcmd__check"
                ;;
            connectors__subcmd__help__subcmd__setup,init)
                cmd="connectors__subcmd__help__subcmd__setup__subcmd__init"
                ;;
            connectors__subcmd__operations,describe)
                cmd="connectors__subcmd__operations__subcmd__describe"
                ;;
            connectors__subcmd__operations,help)
                cmd="connectors__subcmd__operations__subcmd__help"
                ;;
            connectors__subcmd__operations,invoke)
                cmd="connectors__subcmd__operations__subcmd__invoke"
                ;;
            connectors__subcmd__operations,list)
                cmd="connectors__subcmd__operations__subcmd__list"
                ;;
            connectors__subcmd__operations__subcmd__help,describe)
                cmd="connectors__subcmd__operations__subcmd__help__subcmd__describe"
                ;;
            connectors__subcmd__operations__subcmd__help,help)
                cmd="connectors__subcmd__operations__subcmd__help__subcmd__help"
                ;;
            connectors__subcmd__operations__subcmd__help,invoke)
                cmd="connectors__subcmd__operations__subcmd__help__subcmd__invoke"
                ;;
            connectors__subcmd__operations__subcmd__help,list)
                cmd="connectors__subcmd__operations__subcmd__help__subcmd__list"
                ;;
            connectors__subcmd__setup,check)
                cmd="connectors__subcmd__setup__subcmd__check"
                ;;
            connectors__subcmd__setup,help)
                cmd="connectors__subcmd__setup__subcmd__help"
                ;;
            connectors__subcmd__setup,init)
                cmd="connectors__subcmd__setup__subcmd__init"
                ;;
            connectors__subcmd__setup__subcmd__help,check)
                cmd="connectors__subcmd__setup__subcmd__help__subcmd__check"
                ;;
            connectors__subcmd__setup__subcmd__help,help)
                cmd="connectors__subcmd__setup__subcmd__help__subcmd__help"
                ;;
            connectors__subcmd__setup__subcmd__help,init)
                cmd="connectors__subcmd__setup__subcmd__help__subcmd__init"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        connectors)
            opts="-h --config --state-dir --output --help adapters approvals connections operations setup completions help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__adapters)
            opts="-h --config --state-dir --output --help describe list status stop help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__adapters__subcmd__describe)
            opts="-h --adapter --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__adapters__subcmd__help)
            opts="describe list status stop help"
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
        connectors__subcmd__adapters__subcmd__help__subcmd__describe)
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
        connectors__subcmd__adapters__subcmd__help__subcmd__help)
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
        connectors__subcmd__adapters__subcmd__help__subcmd__list)
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
        connectors__subcmd__adapters__subcmd__help__subcmd__status)
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
        connectors__subcmd__adapters__subcmd__help__subcmd__stop)
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
        connectors__subcmd__adapters__subcmd__list)
            opts="-h --limit --cursor --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__adapters__subcmd__status)
            opts="-h --adapter --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__adapters__subcmd__stop)
            opts="-h --adapter --expected-revision --host-incarnation --child-incarnation --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --host-incarnation)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --child-incarnation)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals)
            opts="-h --config --state-dir --output --help clock-check issue key-init key-recover key-retire key-revoke key-rotate key-status policy-set policy-status prepare help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__clock__subcmd__check)
            opts="-h --adapter --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__help)
            opts="clock-check issue key-init key-recover key-retire key-revoke key-rotate key-status policy-set policy-status prepare help"
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
        connectors__subcmd__approvals__subcmd__help__subcmd__clock__subcmd__check)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__help)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__issue)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__init)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__recover)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__retire)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__revoke)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__rotate)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__key__subcmd__status)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__policy__subcmd__set)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__policy__subcmd__status)
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
        connectors__subcmd__approvals__subcmd__help__subcmd__prepare)
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
        connectors__subcmd__approvals__subcmd__issue)
            opts="-h --adapter --connection --operation --schema --revision --input-json --input-file --input-stdin --approve-subject --proof-output --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --connection)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --operation)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --schema)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-json)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --approve-subject)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --proof-output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__key__subcmd__init)
            opts="-h --adapter --expected-revision --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__key__subcmd__recover)
            opts="-h --adapter --expected-revision --candidate --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --candidate)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__key__subcmd__retire)
            opts="-h --adapter --expected-revision --key --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__key__subcmd__revoke)
            opts="-h --adapter --expected-revision --expected-key --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__key__subcmd__rotate)
            opts="-h --adapter --expected-revision --expected-key --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__key__subcmd__status)
            opts="-h --adapter --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__policy__subcmd__set)
            opts="-h --adapter --expected-revision --input-file --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__policy__subcmd__status)
            opts="-h --adapter --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__approvals__subcmd__prepare)
            opts="-h --adapter --connection --operation --schema --revision --input-json --input-file --input-stdin --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --connection)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --operation)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --schema)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-json)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__completions)
            opts="-h --config --state-dir --output --help bash"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__connections)
            opts="-h --config --state-dir --output --help connect describe list repair revalidate revoke status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__connections__subcmd__connect)
            opts="-h --adapter --profile --credential-file --credential-stdin --credential-prompt --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credential-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__connections__subcmd__describe)
            opts="-h --adapter --connection --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --connection)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__connections__subcmd__help)
            opts="connect describe list repair revalidate revoke status help"
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
        connectors__subcmd__connections__subcmd__help__subcmd__connect)
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
        connectors__subcmd__connections__subcmd__help__subcmd__describe)
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
        connectors__subcmd__connections__subcmd__help__subcmd__help)
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
        connectors__subcmd__connections__subcmd__help__subcmd__list)
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
        connectors__subcmd__connections__subcmd__help__subcmd__repair)
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
        connectors__subcmd__connections__subcmd__help__subcmd__revalidate)
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
        connectors__subcmd__connections__subcmd__help__subcmd__revoke)
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
        connectors__subcmd__connections__subcmd__help__subcmd__status)
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
        connectors__subcmd__connections__subcmd__list)
            opts="-h --adapter --limit --cursor --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__connections__subcmd__repair)
            opts="-h --adapter --connection --expected-revision --credential-file --credential-stdin --credential-prompt --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --connection)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --credential-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__connections__subcmd__revalidate)
            opts="-h --adapter --connection --expected-revision --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --connection)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__connections__subcmd__revoke)
            opts="-h --adapter --connection --expected-revision --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --connection)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__connections__subcmd__status)
            opts="-h --adapter --connection --acquisition --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --connection)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --acquisition)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__help)
            opts="adapters approvals connections operations setup completions help"
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
        connectors__subcmd__help__subcmd__adapters)
            opts="describe list status stop"
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
        connectors__subcmd__help__subcmd__adapters__subcmd__describe)
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
        connectors__subcmd__help__subcmd__adapters__subcmd__list)
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
        connectors__subcmd__help__subcmd__adapters__subcmd__status)
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
        connectors__subcmd__help__subcmd__adapters__subcmd__stop)
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
        connectors__subcmd__help__subcmd__approvals)
            opts="clock-check issue key-init key-recover key-retire key-revoke key-rotate key-status policy-set policy-status prepare"
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
        connectors__subcmd__help__subcmd__approvals__subcmd__clock__subcmd__check)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__issue)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__init)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__recover)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__retire)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__revoke)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__rotate)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__key__subcmd__status)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__policy__subcmd__set)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__policy__subcmd__status)
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
        connectors__subcmd__help__subcmd__approvals__subcmd__prepare)
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
        connectors__subcmd__help__subcmd__completions)
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
        connectors__subcmd__help__subcmd__connections)
            opts="connect describe list repair revalidate revoke status"
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
        connectors__subcmd__help__subcmd__connections__subcmd__connect)
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
        connectors__subcmd__help__subcmd__connections__subcmd__describe)
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
        connectors__subcmd__help__subcmd__connections__subcmd__list)
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
        connectors__subcmd__help__subcmd__connections__subcmd__repair)
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
        connectors__subcmd__help__subcmd__connections__subcmd__revalidate)
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
        connectors__subcmd__help__subcmd__connections__subcmd__revoke)
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
        connectors__subcmd__help__subcmd__connections__subcmd__status)
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
        connectors__subcmd__help__subcmd__help)
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
        connectors__subcmd__help__subcmd__operations)
            opts="describe invoke list"
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
        connectors__subcmd__help__subcmd__operations__subcmd__describe)
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
        connectors__subcmd__help__subcmd__operations__subcmd__invoke)
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
        connectors__subcmd__help__subcmd__operations__subcmd__list)
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
        connectors__subcmd__help__subcmd__setup)
            opts="check init"
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
        connectors__subcmd__help__subcmd__setup__subcmd__check)
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
        connectors__subcmd__help__subcmd__setup__subcmd__init)
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
        connectors__subcmd__operations)
            opts="-h --config --state-dir --output --help describe invoke list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__operations__subcmd__describe)
            opts="-h --adapter --operation --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --operation)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__operations__subcmd__help)
            opts="describe invoke list help"
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
        connectors__subcmd__operations__subcmd__help__subcmd__describe)
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
        connectors__subcmd__operations__subcmd__help__subcmd__help)
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
        connectors__subcmd__operations__subcmd__help__subcmd__invoke)
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
        connectors__subcmd__operations__subcmd__help__subcmd__list)
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
        connectors__subcmd__operations__subcmd__invoke)
            opts="-h --adapter --connection --operation --schema --revision --input-json --input-file --input-stdin --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --connection)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --operation)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --schema)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --revision)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-json)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__operations__subcmd__list)
            opts="-h --adapter --limit --cursor --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --adapter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__setup)
            opts="-h --config --state-dir --output --help check init help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__setup__subcmd__check)
            opts="-h --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        connectors__subcmd__setup__subcmd__help)
            opts="check init help"
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
        connectors__subcmd__setup__subcmd__help__subcmd__check)
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
        connectors__subcmd__setup__subcmd__help__subcmd__help)
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
        connectors__subcmd__setup__subcmd__help__subcmd__init)
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
        connectors__subcmd__setup__subcmd__init)
            opts="-h --config --state-dir --output --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --state-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -W "human json" -- "${cur}"))
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
    complete -F _connectors -o nosort -o bashdefault -o default connectors
else
    complete -F _connectors -o bashdefault -o default connectors
fi
