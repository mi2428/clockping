# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_clockping_global_optspecs
	string join \n timestamp= timestamp-format= json C/colored push.url= push.delete-on-exit push.interval= push.job= push.label= push.retries= push.timeout= push.user-agent= metrics.file= metrics.format= metrics.label= metrics.prefix= h/help V/version
end

function __fish_clockping_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_clockping_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_clockping_using_subcommand
	set -l cmd (__fish_clockping_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c clockping -n "__fish_clockping_needs_command" -l timestamp -d 'Timestamp preset for human-readable output' -r -f -a "local\t''
rfc3339\t''
unix\t''
unix-ms\t''
none\t''"
complete -c clockping -n "__fish_clockping_needs_command" -l timestamp-format -d 'strftime-like timestamp format, similar to `date +"..."`' -r
complete -c clockping -n "__fish_clockping_needs_command" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_needs_command" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_needs_command" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_needs_command" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_needs_command" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_needs_command" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_needs_command" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_needs_command" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_needs_command" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_needs_command" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_needs_command" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_needs_command" -l json -d 'Emit JSON Lines instead of text'
complete -c clockping -n "__fish_clockping_needs_command" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_needs_command" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_needs_command" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_needs_command" -s V -l version -d 'Print version'
complete -c clockping -n "__fish_clockping_needs_command" -f -a "icmp" -d 'ICMP echo ping. Native by default; use --pinger to wrap system ping'
complete -c clockping -n "__fish_clockping_needs_command" -f -a "tcp" -d 'TCP connect ping'
complete -c clockping -n "__fish_clockping_needs_command" -f -a "http" -d 'HTTP request ping. HEAD by default; use -X GET to send GET'
complete -c clockping -n "__fish_clockping_needs_command" -f -a "gtp" -d 'GTP Echo ping'
complete -c clockping -n "__fish_clockping_needs_command" -f -a "completion" -d 'Generate a shell completion script'
complete -c clockping -n "__fish_clockping_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_using_subcommand icmp" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -s W -l timeout -d 'Per-probe connect timeout in seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_using_subcommand tcp" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_using_subcommand http" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -s W -l timeout -d 'Per-probe request timeout in seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -s X -l method -d 'HTTP method to send' -r -f -a "head\t''
get\t''"
complete -c clockping -n "__fish_clockping_using_subcommand http" -l ok-status -d 'Treat these HTTP status codes as successful, e.g. 200,204,300-399' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -s H -l header -d 'Add a request header. Repeat for multiple headers' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_using_subcommand http" -s L -l location -d 'Follow HTTP redirects'
complete -c clockping -n "__fish_clockping_using_subcommand http" -s k -l insecure -d 'Skip TLS certificate verification'
complete -c clockping -n "__fish_clockping_using_subcommand http" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_using_subcommand http" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_using_subcommand http" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_using_subcommand http" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -f -a "v1u" -d 'GTPv1-U Echo Request, default UDP/2152'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -f -a "v1c" -d 'GTPv1-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -f -a "v2c" -d 'GTPv2-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and not __fish_seen_subcommand_from v1u v1c v2c help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -s W -l timeout -d 'Per-probe response timeout in seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l port -d 'Override the protocol default UDP port' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1u" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -s W -l timeout -d 'Per-probe response timeout in seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l port -d 'Override the protocol default UDP port' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v1c" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -s c -l count -d 'Stop after count probes. Default is to run until interrupted' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -s i -l interval -d 'Seconds between probes. Fractions are accepted, e.g. 0.2' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -s W -l timeout -d 'Per-probe response timeout in seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -s w -l deadline -d 'Stop the command after this many seconds' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l port -d 'Override the protocol default UDP port' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -s q -l quiet -d 'Suppress per-probe output and only print the summary'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from v2c" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from help" -f -a "v1u" -d 'GTPv1-U Echo Request, default UDP/2152'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from help" -f -a "v1c" -d 'GTPv1-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from help" -f -a "v2c" -d 'GTPv2-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_using_subcommand gtp; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l push.url -d 'Push interval metrics to a Pushgateway URL' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l push.interval -d 'Aggregate interval samples before pushing window metrics' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l push.job -d 'Pushgateway job name' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l push.label -d 'Add a Pushgateway grouping label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l push.retries -d 'Retry failed Pushgateway requests N times' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l push.timeout -d 'Pushgateway request timeout' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l push.user-agent -d 'HTTP User-Agent for Pushgateway requests' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l metrics.file -d 'Write live interval metrics to a file' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l metrics.format -d 'Metrics file format: jsonl or prometheus' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l metrics.label -d 'Add a Prometheus file sample label. Repeat for multiple labels' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l metrics.prefix -d 'Prometheus metric name prefix' -r
complete -c clockping -n "__fish_clockping_using_subcommand completion" -s C -l colored -d 'Colorize human-readable output with ANSI escape sequences'
complete -c clockping -n "__fish_clockping_using_subcommand completion" -l push.delete-on-exit -d 'Delete this Pushgateway grouping key after the run exits'
complete -c clockping -n "__fish_clockping_using_subcommand completion" -s h -l help -d 'Print help'
complete -c clockping -n "__fish_clockping_using_subcommand help; and not __fish_seen_subcommand_from icmp tcp http gtp completion help" -f -a "icmp" -d 'ICMP echo ping. Native by default; use --pinger to wrap system ping'
complete -c clockping -n "__fish_clockping_using_subcommand help; and not __fish_seen_subcommand_from icmp tcp http gtp completion help" -f -a "tcp" -d 'TCP connect ping'
complete -c clockping -n "__fish_clockping_using_subcommand help; and not __fish_seen_subcommand_from icmp tcp http gtp completion help" -f -a "http" -d 'HTTP request ping. HEAD by default; use -X GET to send GET'
complete -c clockping -n "__fish_clockping_using_subcommand help; and not __fish_seen_subcommand_from icmp tcp http gtp completion help" -f -a "gtp" -d 'GTP Echo ping'
complete -c clockping -n "__fish_clockping_using_subcommand help; and not __fish_seen_subcommand_from icmp tcp http gtp completion help" -f -a "completion" -d 'Generate a shell completion script'
complete -c clockping -n "__fish_clockping_using_subcommand help; and not __fish_seen_subcommand_from icmp tcp http gtp completion help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c clockping -n "__fish_clockping_using_subcommand help; and __fish_seen_subcommand_from gtp" -f -a "v1u" -d 'GTPv1-U Echo Request, default UDP/2152'
complete -c clockping -n "__fish_clockping_using_subcommand help; and __fish_seen_subcommand_from gtp" -f -a "v1c" -d 'GTPv1-C Echo Request, default UDP/2123'
complete -c clockping -n "__fish_clockping_using_subcommand help; and __fish_seen_subcommand_from gtp" -f -a "v2c" -d 'GTPv2-C Echo Request, default UDP/2123'
