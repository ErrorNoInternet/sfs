mod commands;
mod utilities;

use commands::{
    get_commands, Context, DecryptCommandConfiguration, EncryptCommandConfiguration,
    LsCommandConfiguration, ParsedCommand, ParsedFlag,
};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::{
    completion::FilenameCompleter, error::ReadlineError, hint::HistoryHinter, Config, Editor,
};
use rustyline_derive::{Completer, Helper, Hinter, Validator};
use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow::{self, Owned};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io::Write};
use utilities::{debug_print, format_colors, generate_fernet, quit_sfs, tokenize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    prompt: String,
    debug_mode: bool,
    list_command: LsCommandConfiguration,
    encrypt_command: EncryptCommandConfiguration,
    decrypt_command: DecryptCommandConfiguration,
}

impl Configuration {
    fn default() -> Self {
        Configuration {
            prompt: String::from("$BOLD$$BLUE$$sfs::path$ >$NORMAL$ "),
            debug_mode: false,
            list_command: LsCommandConfiguration {
                display_all_files: false,
                list_view: false,
                grid_columns: 6,
                decrypt_name: false,
                file_format: String::from("$sfs::name$"),
                folder_format: String::from("$BLUE$$sfs::name$"),
                encrypted_format: String::from("$YELLOW$$sfs::name$"),
                decrypted_name_format: String::from("$YELLOW$$sfs::decrypted_name$ $BOLD$($sfs::name$)")
            },
            encrypt_command: EncryptCommandConfiguration {
                recursive: false,
                silent: false,
                overwrite: false,
                keep_file: false,
                hashing_algorithm: String::from("xxh3"),
                chunk_size: 1048576,
                assign_random_name: false,
                progress_bar_format: String::from(
                    "$BOLD$$sfs::file.name$:$NORMAL$ [{elapsed_precise}] [{wide_bar:.blue/white}] {bytes}/{total_bytes} ({eta})",
                ),
            },
            decrypt_command: DecryptCommandConfiguration {
                recursive: false,
                silent: false,
                overwrite: false,
                keep_file: false,
                use_original_name: false,
                no_verify_chunks: false,
                progress_bar_format: String::from(
                    "$BOLD$$sfs::file.name$:$NORMAL$ [{elapsed_precise}] [{wide_bar:.blue/white}] {bytes}/{total_bytes} ({eta})",
                ),
            },
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
    let mut configuration_path = match home::home_dir() {
        Some(path) => path,
        None => PathBuf::from("."),
    };
    if cfg!(unix) {
        configuration_path.push(".config/sfs")
    } else if cfg!(windows) {
        configuration_path.push("AppData/Roaming/sfs")
    }
    match fs::create_dir_all(&configuration_path) {
        Ok(_) => (),
        Err(error) => {
            println!(
                "{}",
                format_colors(&format!(
                    "$BOLD$Unable to create configuration directory:$NORMAL$ {}",
                    error
                ))
            );
            return;
        }
    };
    let mut configuration_file = configuration_path.clone();
    configuration_file.push("configuration.toml");
    let configuration_string = match fs::read_to_string(&configuration_file) {
        Ok(configuration) => configuration.trim().to_string(),
        Err(_) => String::new(),
    };
    let configuration: Configuration = match toml::from_str(&configuration_string.as_str()) {
        Ok(configuration) => configuration,
        Err(_) => {
            if !configuration_string.is_empty() {
                let mut input = String::new();
                loop {
                    if input.to_lowercase().starts_with("n") {
                        return;
                    } else if input.to_lowercase().starts_with("y") {
                        println!();
                        break;
                    } else {
                        print!("{}", format_colors(&String::from("Your configuration file seems to be corrupted/incomplete. Would you like to overwrite it with a completely new one? $BOLD$Y/N:$NORMAL$ ")));
                        std::io::stdout().flush().unwrap();
                        input.clear();
                        match std::io::stdin().read_line(&mut input) {
                            Ok(_) => (),
                            Err(error) => {
                                println!(
                                    "{} {:?}",
                                    format_colors(&String::from(
                                        "$BOLD$Unable to read input:$NORMAL$"
                                    )),
                                    error
                                );
                                std::process::exit(1)
                            }
                        }
                    }
                }
            }

            let configuration = Configuration::default();
            match fs::write(configuration_file, toml::to_string(&configuration).unwrap()) {
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

    if configuration_string.is_empty() {
        println!("{}", format_colors(&format!("$BOLD$Welcome to the $BLUE$SFS$NORMAL$$BOLD$ shell! This message will only appear once.\n$BOLD$Please enter a password. This password is used to encrypt/decrypt your files, and you must re-enter it every time you launch SFS.\nYou may enter a different password every time you launch SFS, but the files encrypted with your old password won't be accessible.$NORMAL$")));
    }
    print!("Password: ");
    std::io::stdout().flush().unwrap();
    let mut password;
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
    match rpassword::read_password() {
        Ok(repeat_password) => {
            if password != repeat_password {
                println!(
                    "{}",
                    format_colors(&String::from("$BOLD$Passwords do not match!$NORMAL$"))
                );
                return;
            }
        }
        Err(error) => {
            println!(
                "{} {:?}",
                format_colors(&String::from("$BOLD$Unable to read input:$NORMAL$")),
                error,
            );
            std::process::exit(1)
        }
    }
    let fernet = generate_fernet(&password);
    password = String::new();
    password.clear();

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

    let commands = get_commands();
    loop {
        println!("{}", format_colors(&String::new()));

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
            .readline(&format_colors(&configuration.prompt).replace("$sfs::path$", &current_path))
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
            Some(token) => token.to_string(),
            None => continue,
        };
        if first_token.starts_with("!") {
            let mut characters = input.chars();
            characters.next();
            let tokens = tokenize(&characters.as_str().to_string());
            match std::process::Command::new(&tokens[0])
                .args(&tokens[1..])
                .spawn()
            {
                Ok(mut process) => match process.wait() {
                    Ok(_) => (),
                    Err(error) => println!(
                        "{} {:?}",
                        format_colors(&String::from("$BOLD$Process already exited:$NORMAL$")),
                        error,
                    ),
                },
                Err(error) => println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to launch subprocess:$NORMAL$")),
                    error,
                ),
            }
            continue;
        }

        let mut command_found = false;
        for command in &commands {
            let mut matched = false;
            if command.name.to_string() == first_token {
                matched = true;
            } else {
                for alias in command.aliases {
                    if alias.to_string() == first_token {
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
                        for flag in command.flags {
                            if &(String::from("--") + flag.name) == token
                                || &(String::from("-") + &flag.short_name) == token
                            {
                                matched_flag = Some(ParsedFlag {
                                    name: Some(flag.name.to_string()),
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
                                flag.value = Some(token.clone());
                            }
                            parsed_flags.push(flag);
                            matched_flag = None;
                        }
                        None => parsed_flags.push(ParsedFlag {
                            name: None,
                            value: Some(token.clone()),
                        }),
                    };
                }
                if configuration.debug_mode {
                    debug_print(&format!("parsed flags: {:?}", parsed_flags));
                }

                let mut contexts: HashMap<String, Context> = HashMap::new();
                for required_context in command.contexts {
                    match required_context {
                        &"configuration" => contexts.insert(
                            String::from("configuration"),
                            Context::Configuration(configuration.clone()),
                        ),
                        &"fernet" => {
                            contexts.insert(String::from("fernet"), Context::Fernet(fernet.clone()))
                        }
                        _ => None,
                    };
                }

                let command_start = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                (command.callback)(ParsedCommand {
                    name: first_token.clone(),
                    flags: parsed_flags,
                    contexts,
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
