use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::{
    completion::FilenameCompleter, error::ReadlineError, hint::HistoryHinter, Config, Editor,
};
use rustyline_derive::{Completer, Helper, Hinter, Validator};
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow::{self, Owned};
use std::{fs, io::Write};

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
                Err(error) => println!("Unable to save configuration: {}", error),
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
            println!("Unable to read input: {}", error);
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
            println!("Unable to read input: {}", error);
            std::process::exit(1)
        }
    }
    if password == repeat_password {
        let fernet = generate_fernet(&password);
    } else {
        println!("Passwords do not match!");
        return;
    }
    println!();

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
        let current_path;
        match std::env::current_dir() {
            Ok(result) => current_path = result.to_str().unwrap().to_string(),
            Err(error) => {
                println!("Unable to get current working directory: {}", error);
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
                println!("Interrupted");
                input = String::new();
                quit_sfs();
            }
            Err(ReadlineError::Eof) => {
                println!("EOF");
                input = String::new();
                quit_sfs();
            }
            Err(error) => {
                println!("Error: {}", error);
                input = String::new();
                quit_sfs();
            }
        }

        let tokens = tokenize(&input);
        if configuration.debug_mode {
            println!("{:?}", tokens);
        }
        match tokens[0].as_str() {
            "quit" | "exit" | "q" => quit_sfs(),
            "ls" => {
                let mut input_paths = Vec::new();
                for path in tokens.iter().skip(1) {
                    input_paths.push(path.to_owned())
                }
                if input_paths.len() == 0 {
                    input_paths.push(String::from("."))
                }

                for input_path in input_paths {
                    println!(
                        "{}",
                        format_colors(
                            &String::from("$NORMAL$$BOLD$Directory listing for {}$NORMAL$")
                                .replace("{}", &input_path)
                        )
                    );
                    match fs::read_dir(input_path) {
                        Ok(paths) => {
                            for path in paths {
                                match path {
                                    Ok(path) => {
                                        if path.file_type().unwrap().is_dir() {
                                            println!(
                                                "{}",
                                                format_colors(&format!(
                                                    "$BLUE${}",
                                                    path.file_name().to_str().unwrap()
                                                ))
                                            )
                                        } else {
                                            println!(
                                                "{}",
                                                format_colors(&format!(
                                                    "$NORMAL${}",
                                                    path.file_name().to_str().unwrap()
                                                ))
                                            )
                                        }
                                    }
                                    Err(error) => {
                                        println!("Unable to get file information: {}", error)
                                    }
                                }
                            }
                        }
                        Err(error) => println!("Unable to read directory: {}", error),
                    }
                }
            }
            _ => println!("Unknown command"),
        }
        println!()
    }
}

fn tokenize(command: &String) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_string = false;
    for letter in command.chars() {
        if letter == ' ' && !in_string {
            if current_token.len() > 0 {
                tokens.push(current_token);
                current_token = String::new();
            }
            continue;
        }
        if letter == '"' && !in_string {
            in_string = true;
            if current_token.len() > 0 {
                tokens.push(current_token);
                current_token = String::new();
            }
            continue;
        } else if letter == '"' && in_string {
            in_string = false;
            if current_token.len() > 0 {
                tokens.push(current_token);
                current_token = String::new();
            }
            continue;
        }
        current_token.push(letter);
    }
    if current_token.len() > 0 {
        tokens.push(current_token);
    }
    tokens
}

fn generate_fernet(password: &String) -> fernet::Fernet {
    let mut hasher = Sha256::new();
    hasher.update(password.clone().into_bytes());
    let result = format!("{:X}", hasher.finalize());
    let mut key = String::new();
    for (index, letter) in result.chars().enumerate() {
        if index % 2 == 0 {
            key.push(letter);
        }
    }
    fernet::Fernet::new(&base64::encode(key)).unwrap()
}

fn format_colors(text: &String) -> String {
    let mut text = text.clone();
    text = text.replace("$NORMAL$", "\u{001b}[0m");
    text = text.replace("$BOLD$", "\u{001b}[1m");

    text = text.replace("$BLACK$", "\u{001b}[30m");
    text = text.replace("$RED$", "\u{001b}[31m");
    text = text.replace("$GREEN$", "\u{001b}[32m");
    text = text.replace("$YELLOW$", "\u{001b}[33m");
    text = text.replace("$BLUE$", "\u{001b}[34m");
    text = text.replace("$MAGENTA$", "\u{001b}[35m");
    text = text.replace("$CYAN$", "\u{001b}[36m");
    text = text.replace("$WHITE$", "\u{001b}[37m");

    text
}

fn quit_sfs() {
    println!("Quitting...");
    std::process::exit(0)
}
