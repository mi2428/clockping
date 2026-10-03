# Command contexts and option arity come from the CLI schema; dots are literal names.
function __fish_clockping_context
    set -l context clockping
    set -l expecting 0
    set -l positional 0
    for token in (commandline -opc)[2..]
        if test $expecting = 1
            set expecting 0
            continue
        end
        test "$token" = --; and return 1
        set -l options
        set -l value_options
        set -l subcommands
        set -l raw 0
        switch $context
            case 'clockping'
                set options '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
                set subcommands 'icmp' 'tcp' 'http' 'gtp' 'completion' 'help'
            case 'clockping,icmp'
                set options '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
                set raw 1
            case 'clockping,tcp'
                set options '-4' '-6' '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '-q' '--quiet' '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
            case 'clockping,http'
                set options '-4' '-6' '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '-X' '--method' '--ok-status' '-H' '--header' '-L' '--location' '-k' '--insecure' '-q' '--quiet' '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '-X' '--method' '--ok-status' '-H' '--header' '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
            case 'clockping,gtp'
                set options '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
                set subcommands 'v1u' 'v1c' 'v2c' 'help'
            case 'clockping,gtp,v1u'
                set options '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '--port' '-q' '--quiet' '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '--port' '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
            case 'clockping,gtp,v1c'
                set options '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '--port' '-q' '--quiet' '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '--port' '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
            case 'clockping,gtp,v2c'
                set options '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '--port' '-q' '--quiet' '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '-c' '--count' '-i' '--interval' '-W' '--timeout' '-w' '--deadline' '--port' '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
            case 'clockping,gtp,help'
                set subcommands 'v1u' 'v1c' 'v2c' 'help'
            case 'clockping,gtp,help,v1u'
            case 'clockping,gtp,help,v1c'
            case 'clockping,gtp,help,v2c'
            case 'clockping,gtp,help,help'
            case 'clockping,completion'
                set options '--ts.preset' '--ts.format' '--out.format' '--out.colored' '--push.url' '--push.delete-on-exit' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix' '-h' '--help' '-V' '--version'
                set value_options '--ts.preset' '--ts.format' '--out.format' '--push.url' '--push.interval' '--push.job' '--push.label' '--push.retries' '--push.timeout' '--push.user-agent' '--metrics.file' '--metrics.format' '--metrics.label' '--metrics.prefix'
            case 'clockping,help'
                set subcommands 'icmp' 'tcp' 'http' 'gtp' 'completion' 'help'
            case 'clockping,help,icmp'
            case 'clockping,help,tcp'
            case 'clockping,help,http'
            case 'clockping,help,gtp'
                set subcommands 'v1u' 'v1c' 'v2c'
            case 'clockping,help,gtp,v1u'
            case 'clockping,help,gtp,v1c'
            case 'clockping,help,gtp,v2c'
            case 'clockping,help,completion'
            case 'clockping,help,help'
        end
        switch $token
            case '--*'
                set -l option (string split -m 1 = -- $token)[1]
                if not contains -- $option $options
                    test $raw = 1; and continue
                    return 1
                end
                if contains -- $option $value_options; and not string match -q -- '*=*' $token
                    set expecting 1
                end
            case '-?*'
                set -l shorts (string split '' -- (string sub -s 2 -- $token))
                for short in $shorts
                    set -e shorts[1]
                    set -l option -$short
                    if not contains -- $option $options
                        test $raw = 1; and break
                        return 1
                    end
                    if contains -- $option $value_options
                        test (count $shorts) = 0; and set expecting 1
                        break
                    end
                end
            case '*'
                if test $positional = 0; and contains -- $token $subcommands
                    set context $context,$token
                else
                    set positional 1
                end
        end
    end
    test "$context" = "$argv[1]"; or return 1
    if test "$argv[2]" = positional
        test $expecting = 0; and test $positional = 0
    else
        return 0
    end
end

complete -c clockping -n "__fish_clockping_context 'clockping'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping' positional" -f -a 'icmp' -d 'ICMP echo ping. Native by default; use --pinger to wrap system ping'
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping,icmp'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping' positional" -f -a 'tcp' -d 'TCP connect ping'
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s W -l timeout -d 'Per-probe connect timeout in seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s 4 -d 'Use IPv4 only'
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s 6 -d 'Use IPv6 only'
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping,tcp'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping' positional" -f -a 'http' -d 'HTTP request ping. HEAD by default; use -X GET to send GET'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s W -l timeout -d 'Per-probe request timeout in seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s X -l method -d 'HTTP method to send' -r -f -a "head\t''
get\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l ok-status -d 'Treat these HTTP status codes as successful, e.g. 200,204,300-399' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s H -l header -d 'Add a request header. Repeat for multiple headers' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s 4 -d 'Use IPv4 only'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s 6 -d 'Use IPv6 only'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s L -l location -d 'Follow HTTP redirects'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s k -l insecure -d 'Skip TLS certificate verification'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping,http'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping' positional" -f -a 'gtp' -d 'GTP Echo ping'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp' positional" -f -a 'v1u' -d 'GTPv1-U Echo Request, default UDP/2152'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -s W -l timeout -d 'Per-probe response timeout in seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l port -d 'Override the protocol default UDP port' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1u'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp' positional" -f -a 'v1c' -d 'GTPv1-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -s W -l timeout -d 'Per-probe response timeout in seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l port -d 'Override the protocol default UDP port' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v1c'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp' positional" -f -a 'v2c' -d 'GTPv2-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -s W -l timeout -d 'Per-probe response timeout in seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l port -d 'Override the protocol default UDP port' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,v2c'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp' positional" -f -a 'help' -d 'Print this message or the help of the given subcommand(s)'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,help' positional" -f -a 'v1u' -d 'GTPv1-U Echo Request, default UDP/2152'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,help' positional" -f -a 'v1c' -d 'GTPv1-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,help' positional" -f -a 'v2c' -d 'GTPv2-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_context 'clockping,gtp,help' positional" -f -a 'help' -d 'Print this message or the help of the given subcommand(s)'
complete -c clockping -n "__fish_clockping_context 'clockping' positional" -f -a 'completion' -d 'Generate a shell completion script'
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l ts.preset -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l ts.format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l out.format -d 'Output format' -r -f -a "text\t''
json\t''"
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l out.colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c clockping -n "__fish_clockping_context 'clockping,completion'" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_context 'clockping,completion' positional" -f -a 'bash'
complete -c clockping -n "__fish_clockping_context 'clockping,completion' positional" -f -a 'elvish'
complete -c clockping -n "__fish_clockping_context 'clockping,completion' positional" -f -a 'fish'
complete -c clockping -n "__fish_clockping_context 'clockping,completion' positional" -f -a 'powershell'
complete -c clockping -n "__fish_clockping_context 'clockping,completion' positional" -f -a 'zsh'
complete -c clockping -n "__fish_clockping_context 'clockping' positional" -f -a 'help' -d 'Print this message or the help of the given subcommand(s)'
complete -c clockping -n "__fish_clockping_context 'clockping,help' positional" -f -a 'icmp' -d 'ICMP echo ping. Native by default; use --pinger to wrap system ping'
complete -c clockping -n "__fish_clockping_context 'clockping,help' positional" -f -a 'tcp' -d 'TCP connect ping'
complete -c clockping -n "__fish_clockping_context 'clockping,help' positional" -f -a 'http' -d 'HTTP request ping. HEAD by default; use -X GET to send GET'
complete -c clockping -n "__fish_clockping_context 'clockping,help' positional" -f -a 'gtp' -d 'GTP Echo ping'
complete -c clockping -n "__fish_clockping_context 'clockping,help,gtp' positional" -f -a 'v1u' -d 'GTPv1-U Echo Request, default UDP/2152'
complete -c clockping -n "__fish_clockping_context 'clockping,help,gtp' positional" -f -a 'v1c' -d 'GTPv1-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_context 'clockping,help,gtp' positional" -f -a 'v2c' -d 'GTPv2-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_context 'clockping,help' positional" -f -a 'completion' -d 'Generate a shell completion script'
complete -c clockping -n "__fish_clockping_context 'clockping,help' positional" -f -a 'help' -d 'Print this message or the help of the given subcommand(s)'
