use std::fmt::Write;

use clap::Command;
use clap_complete::{Shell, generate};

pub fn script(shell: Shell, command: &mut Command) -> Vec<u8> {
    if shell != Shell::Fish {
        let mut script = Vec::new();
        generate(shell, command, "clockping", &mut script);
        return script;
    }

    command.build();
    let mut contexts = String::new();
    let mut registrations = String::new();
    fish_command(command, "clockping", &mut contexts, &mut registrations);
    format!(
        r#"# Command contexts and option arity come from the CLI schema; dots are literal names.
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
{contexts}        end
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

{registrations}"#
    )
    .into_bytes()
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn fish_command(cmd: &Command, context: &str, contexts: &mut String, registrations: &mut String) {
    writeln!(contexts, "            case {}", quote(context)).unwrap();
    let mut options = Vec::new();
    let mut value_options = Vec::new();
    for arg in cmd.get_arguments().filter(|arg| !arg.is_positional()) {
        let names = arg
            .get_short_and_visible_aliases()
            .into_iter()
            .flatten()
            .map(|short| format!("-{short}"))
            .chain(
                arg.get_long_and_visible_aliases()
                    .into_iter()
                    .flatten()
                    .map(|long| format!("--{long}")),
            );
        for name in names {
            options.push(quote(&name));
            if arg.get_num_args().expect("built CLI").takes_values() {
                value_options.push(quote(&name));
            }
        }
    }
    let subcommands = cmd
        .get_subcommands()
        .flat_map(Command::get_name_and_visible_aliases)
        .map(quote)
        .collect::<Vec<_>>();
    for (name, values) in [
        ("options", options),
        ("value_options", value_options),
        ("subcommands", subcommands),
    ] {
        if !values.is_empty() {
            writeln!(contexts, "                set {name} {}", values.join(" ")).unwrap();
        }
    }
    if cmd
        .get_positionals()
        .any(|arg| arg.is_trailing_var_arg_set())
    {
        writeln!(contexts, "                set raw 1").unwrap();
    }

    // Let clap_complete emit option metadata/escaping, without its argparse-based
    // subcommand helpers (argparse's optspec grammar cannot represent dotted names).
    let mut leaf = Command::new("clockping")
        .version(env!("CARGO_PKG_VERSION"))
        .disable_help_flag(true)
        .disable_version_flag(true)
        .args(cmd.get_arguments().cloned());
    let mut generated = Vec::new();
    generate(Shell::Fish, &mut leaf, "clockping", &mut generated);
    let condition = format!("__fish_clockping_context {}", quote(context));
    registrations.push_str(
        &String::from_utf8(generated)
            .expect("Fish generator emits UTF-8")
            .replace(
                "complete -c clockping",
                &format!("complete -c clockping -n \"{condition}\""),
            ),
    );
    for subcommand in cmd.get_subcommands() {
        for name in subcommand.get_name_and_visible_aliases() {
            writeln!(
                registrations,
                "complete -c clockping -n \"{condition} positional\" -f -a {} -d {}",
                quote(name),
                quote(&subcommand.get_about().unwrap_or_default().to_string())
            )
            .unwrap();
            fish_command(
                subcommand,
                &format!("{context},{name}"),
                contexts,
                registrations,
            );
        }
    }
    // clap_complete's Fish generator only emits named arguments, not positional enums.
    for arg in cmd.get_positionals() {
        if let Some(values) = arg.get_value_parser().possible_values() {
            for value in values.filter(|value| !value.is_hide_set()) {
                writeln!(
                    registrations,
                    "complete -c clockping -n \"{condition} positional\" -f -a {}",
                    quote(value.get_name())
                )
                .unwrap();
            }
        }
    }
}
