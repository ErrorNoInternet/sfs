use crate::utilities::{determine_encrypted_size, format_colors, quit_sfs, remove_colors};
use crate::Configuration;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use serde_derive::{Deserialize, Serialize};
use sfs::{Encrypter, FileMetadata, HashingAlgorithm};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, Write};

#[derive(Clone)]
pub enum Context {
    Configuration(Configuration),
    Fernet(fernet::Fernet),
}

#[derive(Debug, Clone)]
pub struct CommandMetadata {
    pub description: &'static str,
    pub arguments: &'static [&'static str],
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: &'static str,
    pub metadata: CommandMetadata,
    pub flags: &'static [Flag],
    pub aliases: &'static [&'static str],
    pub callback: fn(ParsedCommand),
    pub contexts: &'static [&'static str],
}

#[derive(Clone)]
pub struct ParsedCommand {
    pub name: String,
    pub flags: Vec<ParsedFlag>,
    pub contexts: HashMap<String, Context>,
}

#[derive(Debug, Clone)]
pub struct Flag {
    pub name: &'static str,
    pub short_name: &'static str,
    pub description: &'static str,
    pub has_value: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedFlag {
    pub name: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsCommandConfiguration {
    pub display_all_files: bool,
    pub list_view: bool,
    pub grid_columns: u16,
    pub file_format: String,
    pub folder_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptCommandConfiguration {
    pub silent: bool,
    pub overwrite: bool,
    pub hashing_algorithm: String,
    pub chunk_size: u64,
    pub progress_bar: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptCommandConfiguration {
    pub silent: bool,
    pub overwrite: bool,
    pub no_verify_chunks: bool,
    pub progress_bar: String,
}

pub fn get_commands() -> Vec<Command> {
    let mut commands = Vec::new();
    commands.push(Command {
        name: "version",
        metadata: CommandMetadata {
            description: "Get the current SFS version",
            arguments: &[],
        },
        flags: &[],
        aliases: &["ver", "about"],
        callback: version_command,
        contexts: &[],
    });
    commands.push(Command {
        name: "help",
        metadata: CommandMetadata {
            description: "Get help for a command, or list all commands if none specified",
            arguments: &["(COMMAND)..."],
        },
        flags: &[],
        aliases: &["h", "?"],
        callback: help_command,
        contexts: &[],
    });
    commands.push(Command {
        name: "quit",
        metadata: CommandMetadata {
            description: "Quit SFS",
            arguments: &[],
        },
        flags: &[],
        aliases: &["q", "exit"],
        callback: quit_command,
        contexts: &[],
    });
    commands.push(Command {
        name: "cd",
        metadata: CommandMetadata {
            description: "Change your current working directory",
            arguments: &["[DIRECTORY]"],
        },
        flags: &[],
        aliases: &[],
        callback: change_directory_command,
        contexts: &[],
    });
    commands.push(Command {
        name: "ls",
        metadata: CommandMetadata {
            description:
                "List all the files and folder in the specified directory (defaults to grid view)",
            arguments: &["(DIRECTORY)..."],
        },
        flags: &[
            Flag {
                name: "all",
                short_name: "a",
                description: "List hidden files as well",
                has_value: false,
            },
            Flag {
                name: "list",
                short_name: "l",
                description: "List one file for each line (list view)",
                has_value: false,
            },
            Flag {
                name: "columns",
                short_name: "c",
                description: "The amount of columns to print (for grid view)",
                has_value: true,
            },
        ],
        aliases: &[],
        callback: list_command,
        contexts: &["configuration"],
    });
    commands.push(Command {
        name: "rm",
        metadata: CommandMetadata {
            description: "Remove a file",
            arguments: &["[FILE]..."],
        },
        flags: &[],
        aliases: &["del", "delete"],
        callback: remove_command,
        contexts: &[],
    });
    commands.push(Command {
        name: "cp",
        metadata: CommandMetadata {
            description: "Copy a file to a different location",
            arguments: &["[FILE]", "[DESTINATION]"],
        },
        flags: &[],
        aliases: &["copy"],
        callback: copy_command,
        contexts: &[],
    });
    commands.push(Command {
        name: "mv",
        metadata: CommandMetadata {
            description: "Move a file to a different location",
            arguments: &["[FILE]", "[DESTINATION]"],
        },
        flags: &[],
        aliases: &["move"],
        callback: move_command,
        contexts: &[],
    });
    commands.push(Command {
        name: "clear",
        metadata: CommandMetadata {
            description: "Clear the terminal",
            arguments: &[],
        },
        flags: &[],
        aliases: &["cls"],
        callback: clear_command,
        contexts: &[],
    });
    commands.push(Command {
        name: "encrypt",
        metadata: CommandMetadata {
            description: "Encrypt file(s) with your password",
            arguments: &["[FILE]..."],
        },
        flags: &[
            Flag {
                name: "silent",
                short_name: "s",
                description: "Do not display a progress bar",
                has_value: false,
            },
            Flag {
                name: "overwrite",
                short_name: "o",
                description: "Overwrite the output file even if it exists",
                has_value: false,
            },
            Flag {
                name: "hashing-algorithm",
                short_name: "h",
                description: "Which hashing algorithm to use (none/xxh3)",
                has_value: true,
            },
            Flag {
                name: "chunk-size",
                short_name: "c",
                description: "The size of the encrypted chunks",
                has_value: true,
            },
        ],
        aliases: &[],
        callback: encrypt_command,
        contexts: &["fernet", "configuration"],
    });
    commands.push(Command {
        name: "decrypt",
        metadata: CommandMetadata {
            description: "Decrypt file(s) with your password",
            arguments: &["[FILE]..."],
        },
        flags: &[
            Flag {
                name: "silent",
                short_name: "s",
                description: "Do not display a progress bar",
                has_value: false,
            },
            Flag {
                name: "overwrite",
                short_name: "o",
                description: "Overwrite the output file even if it exists",
                has_value: false,
            },
            Flag {
                name: "no-verify-chunks",
                short_name: "n",
                description: "Don't verify that the chunks match the checksum",
                has_value: false,
            },
            Flag {
                name: "force",
                short_name: "f",
                description: "Decrypt the file even if the file format version doesn't match",
                has_value: false,
            },
        ],
        aliases: &[],
        callback: decrypt_command,
        contexts: &["fernet", "configuration"],
    });
    commands.push(Command {
        name: "information",
        metadata: CommandMetadata {
            description: "Display information about an encrypted file",
            arguments: &["[FILE]..."],
        },
        flags: &[],
        aliases: &["info", "metadata"],
        callback: information_command,
        contexts: &["fernet"],
    });
    commands
}

pub fn version_command(_command: ParsedCommand) {
    println!(
        "{} v{} (file format v{})",
        format_colors(&String::from("$BOLD$SFS$NORMAL$")),
        sfs::SFS_VERSION_STRING,
        sfs::SFS_FORMAT_VERSION,
    )
}

pub fn help_command(command: ParsedCommand) {
    if command.flags.len() > 0 {
        for flag in command.flags {
            if flag.name.is_none() && flag.value.is_some() {
                let input_command = flag.value.unwrap();
                let mut command_found = false;
                for command in get_commands() {
                    let mut matched = false;
                    if command.name == input_command {
                        matched = true;
                    } else {
                        for alias in command.aliases {
                            if alias == &input_command {
                                matched = true;
                            }
                        }
                    }
                    if matched {
                        command_found = true;

                        let mut contexts = String::from("None");
                        if command.contexts.len() > 0 {
                            contexts.clear();
                            contexts = command.contexts.join(", ");
                        }

                        let mut usage = format!("{}", command.name);
                        if command.flags.len() > 0 {
                            usage.push_str(" [FLAG]...")
                        }
                        for argument in command.metadata.arguments {
                            usage.push_str(&(String::from(" ") + &argument))
                        }

                        let mut alias_list = Vec::new();
                        for alias in command.aliases {
                            alias_list.push(format!("$BOLD$`{}`$NORMAL$", alias))
                        }
                        let mut aliases = String::new();
                        if alias_list.len() > 0 {
                            aliases = format!(" (AKA {})", alias_list.join("/"));
                        }

                        let mut flags = String::from(" None");
                        if command.flags.len() > 0 {
                            flags = String::new();
                            for flag in command.flags {
                                let mut has_value = "";
                                if flag.has_value {
                                    has_value = " <value>"
                                }
                                flags += format!(
                                    "\n\t\t$BOLD$-{}$NORMAL$, $BOLD$--{}{}$NORMAL$\n\t\t\t{}",
                                    flag.short_name, flag.name, has_value, flag.description
                                )
                                .as_str()
                            }
                        }

                        println!(
                            "{}",
                            format_colors(&format!(
                                "$BOLD$`{}`$NORMAL${}:\n\t{}\n\n\t$BOLD$Requires:$NORMAL$ {}\n\t$BOLD$Usage:$NORMAL$ {}\n\t$BOLD$Flags:$NORMAL${}",
                                command.name, aliases, command.metadata.description, contexts, usage, flags,
                            ))
                        );
                    }
                }
                if !command_found {
                    println!(
                        "{}",
                        format_colors(&format!(
                            "Unknown command $BOLD$`{}`$NORMAL$. Type $BOLD$`help`$NORMAL$ for a list of commands.",
                            input_command
                        ))
                    )
                }
            }
        }
    } else {
        for command in get_commands() {
            println!(
                "{}",
                format_colors(&format!(
                    "$BOLD$`{}`$NORMAL$ - $BOLD${}$NORMAL$",
                    command.name, command.metadata.description
                ))
            )
        }
    }
}

pub fn quit_command(_: ParsedCommand) {
    quit_sfs();
}

pub fn change_directory_command(command: ParsedCommand) {
    for flag in command.flags {
        if flag.name.is_none() && flag.value.is_some() {
            match std::env::set_current_dir(flag.value.unwrap()) {
                Ok(_) => (),
                Err(error) => {
                    println!(
                        "{} {:?}",
                        format_colors(&String::from("$BOLD$Unable to change directory:$NORMAL$")),
                        error,
                    )
                }
            };
        }
    }
}

pub fn list_command(command: ParsedCommand) {
    let configuration = match command.contexts.get(&String::from("configuration")) {
        Some(configuration) => match configuration {
            Context::Configuration(configuration) => configuration,
            _ => unreachable!(),
        },
        None => {
            println!(
                "{} Configuration was not passed by SFS!",
                format_colors(&String::from("$BOLD$Fatal error:$NORMAL$")),
            );
            return;
        }
    };

    let mut display_all_files = configuration.list_command.display_all_files;
    let mut list_view = configuration.list_command.list_view;
    let mut grid_columns = configuration.list_command.grid_columns;
    let mut input_paths = Vec::new();
    for flag in command.flags {
        if flag.name.is_some() {
            match flag.name.unwrap().as_str() {
                "all" => display_all_files = true,
                "list" => list_view = true,
                "columns" => grid_columns = flag.value.unwrap().parse().unwrap_or(grid_columns),
                _ => (),
            }
        } else if flag.value.is_some() {
            input_paths.push(flag.value.unwrap())
        }
    }
    if grid_columns < 1 {
        grid_columns = 1
    }
    if input_paths.len() == 0 {
        input_paths.push(String::from("."))
    }

    let mut current_column = 0;
    let termsize::Size { cols, rows: _ } = termsize::get().unwrap();
    let padding: usize = (cols / grid_columns).into();
    if padding <= 3 {
        list_view = true;
    }
    let print_file = |path: &std::fs::DirEntry, current_column: &mut u16| {
        let mut file_name = path.file_name().to_str().unwrap().to_string();
        if path.file_type().unwrap().is_dir() {
            file_name = format_colors(&configuration.list_command.folder_format) + &file_name;
        } else {
            file_name = format_colors(&configuration.list_command.file_format) + &file_name;
        }
        let mut colorless_file_name = remove_colors(&file_name);

        if list_view == false {
            if current_column == &grid_columns {
                *current_column = 0;
                println!();
            }

            if colorless_file_name.chars().count() >= padding {
                for _ in 0..colorless_file_name.chars().count() - (padding - 4) {
                    file_name.pop();
                    colorless_file_name.pop();
                }
                file_name += "...";
                colorless_file_name += "..."
            }
            print!(
                "{: <padding$}",
                file_name,
                padding =
                    padding + (file_name.chars().count() - colorless_file_name.chars().count())
            );
            *current_column += 1;
        } else {
            println!("{}", file_name)
        }
    };

    for (index, input_path) in input_paths.iter().enumerate() {
        match fs::read_dir(input_path) {
            Ok(paths) => {
                for path in paths {
                    match path {
                        Ok(path) => {
                            if path.file_name().to_str().unwrap().starts_with(".") {
                                if display_all_files {
                                    print_file(&path, &mut current_column)
                                }
                            } else {
                                print_file(&path, &mut current_column)
                            }
                        }
                        Err(error) => {
                            println!(
                                "{} {:?}",
                                format_colors(&String::from(
                                    "$BOLD$Unable to get file information:$NORMAL$"
                                )),
                                error,
                            )
                        }
                    }
                }
                if !list_view {
                    println!();
                }
                current_column = 0;
            }
            Err(error) => println!(
                "{} {:?}",
                format_colors(&String::from("$BOLD$Unable to read directory:$NORMAL$")),
                error
            ),
        }
        if index != input_paths.len() - 1 {
            println!();
        }
    }
}

pub fn remove_command(command: ParsedCommand) {
    let mut input_paths = Vec::new();
    for flag in command.flags {
        if flag.name.is_none() && flag.value.is_some() {
            input_paths.push(flag.value.unwrap())
        }
    }

    for input_path in input_paths {
        match fs::remove_file(input_path) {
            Ok(_) => (),
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to remove file:$NORMAL$")),
                    error
                );
                continue;
            }
        }
    }
}

pub fn copy_command(command: ParsedCommand) {
    let mut input_paths = Vec::new();
    for flag in command.flags {
        if flag.name.is_none() && flag.value.is_some() {
            input_paths.push(flag.value.unwrap())
        }
    }
    if input_paths.len() <= 1 {
        println!("Not enough arguments!");
        return;
    }

    match fs::copy(
        input_paths.iter().nth(0).unwrap(),
        input_paths.iter().last().unwrap(),
    ) {
        Ok(_) => (),
        Err(error) => {
            println!(
                "{} {:?}",
                format_colors(&String::from("$BOLD$Unable to copy file:$NORMAL$")),
                error
            );
        }
    }
}

pub fn move_command(command: ParsedCommand) {
    let mut input_paths = Vec::new();
    for flag in command.flags {
        if flag.name.is_none() && flag.value.is_some() {
            input_paths.push(flag.value.unwrap())
        }
    }
    if input_paths.len() <= 1 {
        println!("Not enough arguments!");
        return;
    }

    match fs::rename(
        input_paths.iter().nth(0).unwrap(),
        input_paths.iter().last().unwrap(),
    ) {
        Ok(_) => (),
        Err(error) => {
            println!(
                "{} {:?}",
                format_colors(&String::from("$BOLD$Unable to move file:$NORMAL$")),
                error
            );
        }
    }
}

pub fn clear_command(_command: ParsedCommand) {
    print!("\u{001b}[2J\u{001b}[H")
}

pub fn encrypt_command(command: ParsedCommand) {
    let fernet = match command.contexts.get(&String::from("fernet")) {
        Some(fernet) => match fernet {
            Context::Fernet(fernet) => fernet,
            _ => unreachable!(),
        },
        None => {
            println!(
                "{} Fernet was not passed by SFS!",
                format_colors(&String::from("$BOLD$Fatal error:$NORMAL$")),
            );
            return;
        }
    };
    let configuration = match command.contexts.get(&String::from("configuration")) {
        Some(configuration) => match configuration {
            Context::Configuration(configuration) => configuration,
            _ => unreachable!(),
        },
        None => {
            println!(
                "{} Configuration was not passed by SFS!",
                format_colors(&String::from("$BOLD$Fatal error:$NORMAL$")),
            );
            return;
        }
    };

    let mut silent = configuration.encrypt_command.silent;
    let mut overwrite = configuration.encrypt_command.overwrite;
    let mut input_hashing_algorithm = configuration.encrypt_command.hashing_algorithm.clone();
    let mut chunk_size = configuration.encrypt_command.chunk_size;
    let mut input_paths = Vec::new();
    for flag in command.flags {
        if flag.name.is_some() {
            match flag.name.unwrap().as_str() {
                "silent" => silent = true,
                "overwrite" => overwrite = true,
                "hashing-algorithm" => input_hashing_algorithm = flag.value.unwrap().to_owned(),
                "chunk-size" => chunk_size = flag.value.unwrap().parse().unwrap_or(chunk_size),
                _ => (),
            }
        } else if flag.value.is_some() {
            input_paths.push(flag.value.unwrap())
        }
    }
    let hashing_algorithm = match input_hashing_algorithm.to_lowercase().as_str() {
        "none" => HashingAlgorithm::None,
        "xxh3" => HashingAlgorithm::Xxh3,
        _ => {
            println!("Unknown hashing algorithm, defaulting to none");
            HashingAlgorithm::None
        }
    };

    'input_loop: for input_path in input_paths {
        let input_file = match fs::File::open(&input_path) {
            Ok(file) => file,
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to open file:$NORMAL$")),
                    error
                );
                continue;
            }
        };
        let mut buffered_reader = BufReader::new(&input_file);
        let output_path = input_path.to_string() + ".sfs";
        if !overwrite {
            if fs::metadata(&output_path).is_ok() {
                let mut input = String::new();
                loop {
                    if input.to_lowercase().starts_with("n") {
                        return;
                    } else if input.to_lowercase().starts_with("y") {
                        break;
                    } else {
                        print!(
                        "{}",
                        format_colors(&format!("$BOLD${}$NORMAL$ already exists. Do you want to overwrite it? $BOLD$Y/N:$NORMAL$ ", output_path))
                    );
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
        }
        let mut output_file = match fs::File::create(output_path) {
            Ok(file) => file,
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to create file:$NORMAL$")),
                    error
                );
                continue;
            }
        };

        let mut encrypter = Encrypter::new(fernet.to_owned(), &hashing_algorithm);
        let mut buffer = vec![0; chunk_size as usize];
        let file_size = match input_file.metadata() {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to get file metadata:$NORMAL$")),
                    error
                );
                continue 'input_loop;
            }
        };
        let progress_bar = ProgressBar::new(file_size);
        progress_bar.set_style(
            ProgressStyle::with_template(configuration.encrypt_command.progress_bar.as_str())
                .unwrap()
                .with_key(
                    "eta",
                    |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                        write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
                    },
                )
                .progress_chars("#>-"),
        );

        let encrypted_size = determine_encrypted_size(FileMetadata::default().pack().len());
        match output_file.write(&vec![Default::default(); encrypted_size]) {
            Ok(_) => (),
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from(
                        "$BOLD$Unable to write metadata padding:$NORMAL$"
                    )),
                    error
                );
                continue 'input_loop;
            }
        }
        loop {
            let read = match buffered_reader.read(&mut buffer) {
                Ok(read) => read,
                Err(error) => {
                    println!(
                        "{} {:?}",
                        format_colors(&String::from("$BOLD$Unable to read chunk:$NORMAL$")),
                        error
                    );
                    continue 'input_loop;
                }
            };
            if read == 0 {
                break;
            }

            let mut encrypted = "\n".as_bytes().to_vec();
            encrypted.append(&mut encrypter.encrypt(&buffer[..read]).into_bytes());
            match output_file.write(&encrypted) {
                Ok(_) => (),
                Err(error) => {
                    println!(
                        "{} {:?}",
                        format_colors(&String::from("$BOLD$Unable to write chunk:$NORMAL$")),
                        error
                    );
                    continue 'input_loop;
                }
            }

            if !silent {
                progress_bar.set_position(encrypter.total_bytes)
            }
        }
        match output_file.seek(std::io::SeekFrom::Start(0)) {
            Ok(_) => (),
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to seek file:$NORMAL$")),
                    error
                );
                continue 'input_loop;
            }
        };
        match output_file.write(
            &fernet
                .encrypt(
                    &FileMetadata {
                        format_version: sfs::SFS_FORMAT_VERSION,
                        hashing_algorithm: hashing_algorithm as u8,
                        checksum: encrypter.get_checksum(),
                        total_bytes: encrypter.total_bytes,
                        chunk_size,
                    }
                    .pack(),
                )
                .into_bytes(),
        ) {
            Ok(_) => (),
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to write metadata:$NORMAL$")),
                    error
                );
                continue 'input_loop;
            }
        }
    }
}

pub fn decrypt_command(_command: ParsedCommand) {}

pub fn information_command(command: ParsedCommand) {
    let fernet = match command.contexts.get(&String::from("fernet")) {
        Some(fernet) => match fernet {
            Context::Fernet(fernet) => fernet,
            _ => unreachable!(),
        },
        None => {
            println!(
                "{} Fernet was not passed by SFS!",
                format_colors(&String::from("$BOLD$Fatal error:$NORMAL$")),
            );
            return;
        }
    };

    let mut input_paths = Vec::new();
    for flag in command.flags {
        if flag.name.is_none() && flag.value.is_some() {
            input_paths.push(flag.value.unwrap())
        }
    }

    for input_path in input_paths {
        let input_file = match fs::File::open(&input_path) {
            Ok(file) => file,
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to open file:$NORMAL$")),
                    error
                );
                continue;
            }
        };

        let mut buffer = String::new();
        match BufReader::new(&input_file).read_line(&mut buffer) {
            Ok(_) => buffer = buffer.trim().to_string(),
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to read metadata:$NORMAL$")),
                    error
                );
                continue;
            }
        }
        let metadata_bytes = match fernet.decrypt(&buffer) {
            Ok(metadata_bytes) => metadata_bytes,
            Err(error) => {
                println!(
                    "{} {:?} (incorrect password?)",
                    format_colors(&String::from("$BOLD$Unable to decrypt metadata:$NORMAL$")),
                    error
                );
                continue;
            }
        };
        let metadata = match FileMetadata::parse(&metadata_bytes) {
            Ok(metadata) => metadata,
            Err(error) => {
                println!(
                    "{} {:?}",
                    format_colors(&String::from("$BOLD$Unable to unpack metadata:$NORMAL$")),
                    error
                );
                continue;
            }
        };

        println!(
            "{}",
            format_colors(&format!(
                "$BOLD$`{}`$NORMAL$:\n\t$BOLD$SFS Format Version:$NORMAL$ {}\n\t$BOLD$Decrypted Size:$NORMAL$ {} ({})\n\t$BOLD$Hashing Algorithm:$NORMAL$ {}\n\t$BOLD$Checksum:$NORMAL$ {:X}\n\t$BOLD$Chunk Size:$NORMAL$ {} ({})",
                input_path,
                metadata.format_version,
                metadata.total_bytes,
                humansize::format_size(metadata.total_bytes, humansize::DECIMAL),
                HashingAlgorithm::from_u8(metadata.hashing_algorithm),
                metadata.checksum,
                metadata.chunk_size,
                humansize::format_size(metadata.chunk_size, humansize::DECIMAL),
            ))
        )
    }
}
