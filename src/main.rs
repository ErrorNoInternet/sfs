use serde_derive::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Write};

#[derive(Deserialize)]
struct Configuration {
    prompt: String,
    security: SecurityConfiguration,
}

impl Configuration {
    fn default() -> Self {
        Configuration {
            prompt: String::from("$PATH$ $BOLD$>$NORMAL$ "),
            security: SecurityConfiguration {
                password_timeout_seconds: 300,
            },
        }
    }
}

#[derive(Deserialize)]
struct SecurityConfiguration {
    password_timeout_seconds: i32,
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
        Err(_) => Configuration::default(),
    };

    print!("Password (for this session): ");
    std::io::stdout().flush();
    let password = rpassword::read_password().expect("Unable to read password");
    if password.len() <= 0 {
        println!("No password specified. Quitting...");
        return;
    }
    let fernet = generate_fernet(&password);

    loop {
        let mut current_path = String::new();
        match std::env::current_dir() {
            Ok(current_pathbuf) => current_path = current_pathbuf.to_str().unwrap().to_string(),
            Err(error) => {
                println!("Unable to get current working directory: {}", error);
                return;
            }
        }
        let mut prompt = format_prompt(&configuration.prompt);
        prompt = prompt.replace("$PATH$", &current_path);
        print!("{}", prompt);
        std::io::stdout().flush();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input);
        input = input.trim().to_string();
        let tokens = tokenize(&input);

        match tokens[0].as_str() {
            "ls" => {
                let current_directory = String::from(".");
                let input_path = tokens.iter().nth(1).unwrap_or(&current_directory);
                match fs::read_dir(input_path) {
                    Ok(paths) => {
                        for path in paths {
                            println!(
                                "{}",
                                path.unwrap()
                                    .path()
                                    .display()
                                    .to_string()
                                    .trim_start_matches((input_path.to_owned() + "/").as_str())
                            )
                        }
                    }
                    Err(error) => {
                        println!("Unable to read directory: {}", error);
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
            tokens.push(current_token);
            current_token = String::new();
            continue;
        }
        if letter == '"' && !in_string {
            in_string = true;
            continue;
        } else if letter == '"' && in_string {
            in_string = false;
            continue;
        }
        current_token.push(letter);
    }
    tokens.push(current_token);
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

fn format_prompt(old_prompt: &String) -> String {
    let mut prompt = old_prompt.clone();
    prompt = prompt.replace("$NORMAL$", "\u{001b}[0m");
    prompt = prompt.replace("$BOLD$", "\u{001b}[1m");
    prompt
}
