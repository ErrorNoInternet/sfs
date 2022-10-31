use crate::utilities::{format_colors, quit_sfs};
use crate::Configuration;

use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fs;

pub enum Context {
    Configuration(Configuration),
    Fernet(fernet::Fernet),
}

#[derive(Debug)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub flags: Vec<Flag>,
    pub aliases: Vec<String>,
    pub callback: fn(ParsedCommand),
    pub contexts: Vec<String>,
}

pub struct ParsedCommand {
    pub name: String,
    pub flags: Vec<ParsedFlag>,
    pub contexts: HashMap<String, Context>,
}

#[derive(Debug)]
pub struct Flag {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub has_value: bool,
}

#[derive(Debug)]
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

pub fn get_commands() -> Vec<Command> {
    let mut commands = Vec::new();
    commands.push(Command {
        name: String::from("help"),
        description: String::from("Get help for a command, or list all commands if none specified"),
        flags: Vec::new(),
        aliases: vec![String::from("h"), String::from("?")],
        callback: help_command,
        contexts: Vec::new(),
    });
    commands.push(Command {
        name: String::from("quit"),
        description: String::from("Quit SFS"),
        flags: Vec::new(),
        aliases: vec![String::from("q"), String::from("exit")],
        callback: quit_command,
        contexts: Vec::new(),
    });
    commands.push(Command {
        name: String::from("cd"),
        description: String::from("Change your current working directory"),
        flags: Vec::new(),
        aliases: Vec::new(),
        callback: cd_command,
        contexts: Vec::new(),
    });
    commands.push(Command {
        name: String::from("ls"),
        description: String::from(
            "List all the files and folder in the specified directory (grid view)",
        ),
        flags: vec![
            Flag {
                name: String::from("all"),
                short_name: String::from("a"),
                description: String::from("List hidden files as well"),
                has_value: false,
            },
            Flag {
                name: String::from("list"),
                short_name: String::from("l"),
                description: String::from("List one file for each line (list view)"),
                has_value: false,
            },
            Flag {
                name: String::from("columns"),
                short_name: String::from("c"),
                description: String::from("The amount of columns to print (for grid view)"),
                has_value: true,
            },
        ],
        aliases: Vec::new(),
        callback: ls_command,
        contexts: vec![String::from("configuration")],
    });
    commands.push(Command {
        name: String::from("encrypt"),
        description: String::from("Encrypt file(s) with your password"),
        flags: vec![Flag {
            name: String::from("silent"),
            short_name: String::from("-s"),
            description: String::from("Do not display a progress bar"),
            has_value: false,
        }],
        aliases: Vec::new(),
        callback: encrypt_command,
        contexts: vec![String::from("fernet")],
    });
    commands.push(Command {
        name: String::from("decrypt"),
        description: String::from("Decrypt file(s) with your password"),
        flags: vec![Flag {
            name: String::from("silent"),
            short_name: String::from("-s"),
            description: String::from("Do not display a progress bar"),
            has_value: false,
        }],
        aliases: Vec::new(),
        callback: decrypt_command,
        contexts: vec![String::from("fernet")],
    });
    commands
}

pub fn help_command(command: ParsedCommand) {
    if command.flags.len() > 0 {
        for flag in command.flags {
            if flag.value.is_some() {
                let input_command = flag.value.unwrap();
                let mut command_found = false;
                for command in get_commands() {
                    let mut matched = false;
                    if command.name == input_command {
                        matched = true;
                    } else {
                        for alias in &command.aliases {
                            if alias == &input_command {
                                matched = true;
                            }
                        }
                    }
                    if matched {
                        command_found = true;

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
                        }
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
                        println!(
                            "{}",
                            format_colors(&format!(
                                "$BOLD$`{}`$NORMAL${}:\n\t{}\n\n\t$BOLD$Flags$NORMAL$:{}",
                                command.name, aliases, command.description, flags,
                            ))
                        );
                    }
                }
                if !command_found {
                    println!(
                    "{}",
                    format_colors(&format!(
                        "Unknown command $BOLD$`{}`$NORMAL$. Type $BOLD$`help`$NORMAL$ for a list of commands.",
                        input_command.as_str()
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
                    command.name, command.description
                ))
            )
        }
    }
}

pub fn quit_command(_: ParsedCommand) {
    quit_sfs();
}

pub fn cd_command(command: ParsedCommand) {
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

pub fn ls_command(command: ParsedCommand) {
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

    let mut display_all_files = configuration.ls_command.display_all_files;
    let mut list_view = configuration.ls_command.list_view;
    let mut grid_columns = configuration.ls_command.grid_columns;
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
        if list_view == false {
            if current_column == &grid_columns {
                *current_column = 0;
                println!();
            }

            let mut file_name = path.file_name().to_str().unwrap().to_string();
            if file_name.len() >= padding {
                while file_name.len() >= padding - 3 {
                    file_name.pop();
                }
                file_name += "..."
            }
            if path.file_type().unwrap().is_dir() {
                print!(
                    "{}{: <padding$}",
                    format_colors(&configuration.ls_command.folder_format),
                    file_name,
                    padding = padding,
                )
            } else {
                print!(
                    "{}{: <padding$}",
                    format_colors(&configuration.ls_command.file_format),
                    file_name,
                    padding = padding,
                )
            }
            *current_column += 1;
        } else {
            if path.file_type().unwrap().is_dir() {
                println!(
                    "{}{}",
                    format_colors(&configuration.ls_command.folder_format),
                    path.file_name().to_str().unwrap()
                )
            } else {
                println!(
                    "{}{}",
                    format_colors(&configuration.ls_command.file_format),
                    path.file_name().to_str().unwrap()
                )
            }
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

pub fn encrypt_command(command: ParsedCommand) {
    let _fernet = match command.contexts.get(&String::from("fernet")) {
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
}

pub fn decrypt_command(command: ParsedCommand) {
    let _fernet = match command.contexts.get(&String::from("fernet")) {
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
}
