mod commands;
mod utilities;

use commands::get_commands;
use utilities::{debug_print, format_colors, generate_fernet, quit_sfs, tokenize};

use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::{
    completion::FilenameCompleter, error::ReadlineError, hint::HistoryHinter, Config, Editor,
};
use rustyline_derive::{Completer, Helper, Hinter, Validator};
use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow::{self, Owned};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io::Write};

use crate::commands::{ParsedCommand, ParsedFlag};

#[derive(Debug, Serialize, Deserialize)]
struct Configuration {
    prompt: String,
    debug_mode: bool,
}

impl Configuration {
    fn default() -> Self {
        Configuration {
            prompt: String::from("$BOLD$$BLUE$$PATH$ >$NORMAL$ "),
            debug_mode: false,
        }
    }
}

#[derive(Helper, Completer, Hinter, Validator)]
struct AutocompleteHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    highlighter: MatchingBracketHighlighter,
}

impl Highlighter for AutocompleteHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

fn main() {
    let mut configuration_path = String::from("./");
    if cfg!(unix) {
        configuration_path = format!("/home/{}/.config/sfs/", whoami::username());
    } else if cfg!(windows) {
        configuration_path = format!("C:/Users/{}/AppData/Roaming/sfs/", whoami::username())
    }
    fs::create_dir_all(&configuration_path).expect("Unable to create configuration directory");
    let configuration_string =
        match fs::read_to_string(configuration_path.to_owned() + "configuration.toml") {
            Ok(configuration) => configuration,
            Err(_) => String::new(),
        };
    let configuration: Configuration = match toml::from_str(&configuration_string.as_str()) {
        Ok(configuration) => configuration,
        Err(_) => {
            let configuration = Configuration::default();
            match fs::write(
                configuration_path.to_owned() + "configuration.toml",
                toml::to_string(&configuration).unwrap(),
            ) {
                Ok(_) => (),
                Err(error) => println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to save configuration:$NORMAL$")),
                    error,
                ),
            };
            configuration
        }
    };

    print!("{}\nPassword: ", format_colors(&String::from("$BOLD$Please enter your password (used to encrypt/decrypt files, only for this session).$NORMAL$")));
    std::io::stdout().flush().unwrap();
    let password;
    match rpassword::read_password() {
        Ok(result) => password = result,
        Err(error) => {
            println!(
                "{} {:?}",
                format_colors(&String::from("$BOLD$Unable to read input:$NORMAL$")),
                error
            );
            std::process::exit(1)
        }
    }
    if password.len() <= 0 {
        println!("No password specified. Quitting...");
        std::process::exit(1)
    }
    print!("Repeat Password: ");
    std::io::stdout().flush().unwrap();
    let repeat_password;
    match rpassword::read_password() {
        Ok(result) => repeat_password = result,
        Err(error) => {
            println!(
                "{} {:?}",
                format_colors(&String::from("$BOLD$Unable to read input:$NORMAL$")),
                error,
            );
            std::process::exit(1)
        }
    }
    if password == repeat_password {
        let fernet = generate_fernet(&password);
    } else {
        println!("Passwords do not match!");
        return;
    }

    let commands = get_commands();

    let editor_configuration = Config::builder()
        .history_ignore_space(true)
        .completion_type(rustyline::CompletionType::List)
        .build();
    let mut editor = Editor::with_config(editor_configuration).unwrap();
    let autocomplete_helper = AutocompleteHelper {
        completer: FilenameCompleter::new(),
        hinter: HistoryHinter {},
        highlighter: MatchingBracketHighlighter::new(),
    };
    editor.set_helper(Some(autocomplete_helper));
    loop {
        println!();

        let current_path;
        match std::env::current_dir() {
            Ok(result) => current_path = result.to_str().unwrap().to_string(),
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from(
                        "$BOLD$Unable to get current working directory:$NORMAL$"
                    )),
                    error,
                );
                std::process::exit(1)
            }
        }
        let input;
        match editor
            .readline(&format_colors(&configuration.prompt).replace("$PATH$", &current_path))
        {
            Ok(mut line) => {
                line = line.trim().to_string();
                editor.add_history_entry(&line);
                input = line;
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupted!");
                input = String::new();
            }
            Err(ReadlineError::Eof) => {
                println!("EOF!");
                input = String::new();
                quit_sfs();
            }
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Error:$NORMAL$")),
                    error
                );
                input = String::new();
                quit_sfs();
            }
        }

        let tokens = tokenize(&input);
        if configuration.debug_mode {
            debug_print(&format!("tokens: {:?}", tokens));
        }
        let first_token = match tokens.iter().nth(0) {
            Some(token) => token,
            None => continue,
        };

        let mut command_found = false;
        for command in &commands {
            let mut matched = false;
            if &command.name == first_token {
                matched = true;
            } else {
                for alias in &command.aliases {
                    if alias == first_token {
                        matched = true;
                    }
                }
            }
            if matched {
                command_found = true;
                if configuration.debug_mode {
                    debug_print(&format!("matched command: {:?}", command));
                }

                let mut parsed_flags = Vec::new();
                let mut matched_flag: Option<ParsedFlag> = None;
                let mut looking_for_value = false;
                'token_loop: for token in tokens.iter().skip(1) {
                    if !looking_for_value {
                        for flag in &command.flags {
                            if &(String::from("--") + &flag.name) == token
                                || &(String::from("-") + &flag.short_name) == token
                            {
                                matched_flag = Some(ParsedFlag {
                                    name: Some(flag.name.clone()),
                                    value: None,
                                });
                                if flag.has_value {
                                    looking_for_value = true;
                                    continue 'token_loop;
                                }
                            }
                        }
                    }

                    match matched_flag {
                        Some(mut flag) => {
                            if looking_for_value {
                                looking_for_value = false;
                                flag.value = Some(token.to_string());
                            }
                            parsed_flags.push(flag);
                            matched_flag = None;
                        }
                        None => parsed_flags.push(ParsedFlag {
                            name: None,
                            value: Some(token.to_string()),
                        }),
                    };
                }
                if configuration.debug_mode {
                    debug_print(&format!("parsed flags: {:?}", parsed_flags));
                }

                let command_start = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                (command.callback)(ParsedCommand {
                    name: first_token.to_string(),
                    flags: parsed_flags,
                });
                let command_end = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                if configuration.debug_mode {
                    debug_print(&format!("command took {} ms", command_end - command_start))
                }
            }
        }

        if !command_found {
            println!(
                "{}",
                format_colors(&String::from(
                    "$BOLD$Unknown command!$NORMAL$ Type $BOLD$`help`$NORMAL$ for a list of commands."
                ))
            );
        }
    }
}
