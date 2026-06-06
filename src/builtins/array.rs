use crate::array::ArrayValue;
use crate::shell::Shell;

impl Shell {
    pub(crate) fn handle_array_command(&mut self, args: &[String]) {
        if args.is_empty() {
            println!("Array commands: define, get, len, list");
            return;
        }

        match args[0].as_str() {
            "define" => {
                if args.len() > 2 {
                    let array_name = &args[1];
                    let elements: Vec<String> = args[2..].to_vec();
                    self.env_vars
                        .insert(array_name.to_string(), ArrayValue::Array(elements));
                    println!(
                        "Array '{}' defined with {} elements",
                        array_name,
                        args.len() - 2
                    );
                }
            }
            "get" => {
                if args.len() > 2 {
                    let array_name = &args[1];
                    let index: usize = args[2].parse().unwrap_or(0);
                    if let Some(ArrayValue::Array(arr)) = self.env_vars.get(array_name) {
                        if let Some(element) = arr.get(index) {
                            println!("{}", element);
                        } else {
                            println!("Index out of bounds");
                        }
                    } else {
                        println!("Array '{}' not found", array_name);
                    }
                }
            }
            "len" => {
                if args.len() > 1 {
                    let array_name = &args[1];
                    if let Some(ArrayValue::Array(arr)) = self.env_vars.get(array_name) {
                        println!("{}", arr.len());
                    } else {
                        println!("Array '{}' not found", array_name);
                    }
                }
            }
            "list" => {
                for (key, value) in &self.env_vars {
                    if let ArrayValue::Array(arr) = value {
                        println!("{}=({})", key, arr.join(" "));
                    }
                }
            }
            _ => {
                println!("Array commands: define, get, len, list");
            }
        }
    }
}
