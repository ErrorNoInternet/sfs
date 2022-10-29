use crate::utilities::{format_colors, quit_sfs};

use std::fs;

#[derive(Debug)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub flags: Vec<Flag>,
    pub aliases: Vec<String>,
    pub callback: fn(ParsedCommand),
}

#[derive(Debug)]
pub struct ParsedCommand {
    pub name: String,
    pub flags: Vec<ParsedFlag>,
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
    let mut display_all_files = false;
    let mut list_view = false;
    let mut grid_columns = 5;
    let mut input_paths = Vec::new();

    for flag in command.flags {
        if flag.name.is_some() {
            match flag.name.unwrap().as_str() {
                "all" => display_all_files = true,
                "list" => list_view = true,
                "columns" => grid_columns = flag.value.unwrap().parse().unwrap_or(5),
                _ => (),
            }
        } else if flag.value.is_some() {
            input_paths.push(flag.value.unwrap())
        }
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
    let mut print_file = |path: &std::fs::DirEntry| {
        if list_view == false {
            if current_column == grid_columns {
                current_column = 0;
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
                    format_colors(&String::from("$BLUE$")),
                    file_name,
                    padding = padding,
                )
            } else {
                print!(
                    "{}{: <padding$}",
                    format_colors(&String::from("")),
                    file_name,
                    padding = padding,
                )
            }
            current_column += 1;
        } else {
            if path.file_type().unwrap().is_dir() {
                println!(
                    "{}",
                    format_colors(&format!("$BLUE${}", path.file_name().to_str().unwrap()))
                )
            } else {
                println!(
                    "{}",
                    format_colors(&format!("{}", path.file_name().to_str().unwrap()))
                )
            }
        }
    };

    for input_path in input_paths {
        match fs::read_dir(input_path) {
            Ok(paths) => {
                for path in paths {
                    match path {
                        Ok(path) => {
                            if path.file_name().to_str().unwrap().starts_with(".") {
                                if display_all_files {
                                    print_file(&path)
                                }
                            } else {
                                print_file(&path)
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
            }
            Err(error) => println!(
                "{} {:?}",
                format_colors(&String::from("$BOLD$Unable to read directory:$NORMAL$ {}")),
                error
            ),
        }
        if !list_view {
            println!();
        }
    }
}
