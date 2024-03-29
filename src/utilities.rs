use fernet::Fernet;
use sha2::{Digest, Sha256};

pub fn determine_encrypted_size(input_size: usize) -> usize {
    Fernet::new(&Fernet::generate_key())
        .unwrap()
        .encrypt(&"A".repeat(input_size).into_bytes())
        .len()
}

pub fn debug_print(message: &String) {
    println!(
        "{} {}",
        format_colors(&String::from("$BOLD$[DEBUG]$NORMAL$")),
        message
    )
}

pub fn format_colors(text: &String) -> String {
    let mut text = String::from("$NORMAL$") + &text.clone();

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

pub fn remove_colors(text: &String) -> String {
    let mut text = text.clone();

    text = text.replace("\u{001b}[0m", "");
    text = text.replace("\u{001b}[1m", "");

    text = text.replace("\u{001b}[30m", "");
    text = text.replace("\u{001b}[31m", "");
    text = text.replace("\u{001b}[32m", "");
    text = text.replace("\u{001b}[33m", "");
    text = text.replace("\u{001b}[34m", "");
    text = text.replace("\u{001b}[35m", "");
    text = text.replace("\u{001b}[36m", "");
    text = text.replace("\u{001b}[37m", "");

    text
}

pub fn generate_fernet(password: &String) -> fernet::Fernet {
    let mut result = password.to_owned();
    for _ in 0..256 {
        let mut hasher = Sha256::new();
        hasher.update(result.clone().into_bytes());
        result = format!("{:X}", hasher.finalize());
    }

    let mut key = String::new();
    for (index, letter) in result.chars().enumerate() {
        if index % 2 == 0 {
            key.push(letter);
        }
    }

    fernet::Fernet::new(&base64::encode_config(key, base64::URL_SAFE)).unwrap()
}

pub fn tokenize(command: &String) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_string = (false, "");
    for letter in command.chars() {
        if letter == '\\' {
            in_string = (true, "\\");
            continue;
        }
        if letter == ' ' && !in_string.0 {
            if current_token.len() > 0 {
                tokens.push(current_token);
                current_token = String::new();
            }
            continue;
        }
        if letter == '"' && !in_string.0 {
            in_string = (true, "\"");
            if current_token.len() > 0 {
                tokens.push(current_token);
                current_token = String::new();
            }
            continue;
        } else if letter == '"' && in_string.0 {
            in_string = (false, "");
            if current_token.len() > 0 {
                tokens.push(current_token);
                current_token = String::new();
            }
            continue;
        }
        current_token.push(letter);
        if in_string == (true, "\\") {
            in_string = (false, "");
        }
    }
    if current_token.len() > 0 {
        tokens.push(current_token);
    }
    tokens
}

pub fn quit_sfs() {
    println!("Quitting SFS...");
    std::process::exit(0)
}
