use std::collections::HashMap;
use crate::parser::vars::{Context, NativeFunction, Variable, variables_to_string};
use anyhow::{Result, bail};

pub fn get_native_functions() -> HashMap<String, NativeFunction> {
    let mut map = HashMap::new();

    fn rush_trim(_ctx: &mut Context, args: Vec<Variable>) -> Result<Variable> {
        let text = variables_to_string(args);
        let trimmed = text.trim();
        Ok(Variable::String(trimmed.to_string()))
    }
    map.insert("$trim".to_string(), NativeFunction {
        name: "$trim".to_string(),
        description: "Removes leading and trailing whitespaces from a string".to_string(),
        args: vec![String::from("str")],
        func: rush_trim
    });

    fn rush_test(_ctx: &mut Context, args: Vec<Variable>) -> Result<Variable> {
        if args.len() != 3 {
            bail!("Expected 3 arguments (source, operand, target), got {}", args.len());
        }
        let source = args.get(0).unwrap();
        let operand = args.get(1).unwrap();
        let target = args.get(2).unwrap();

        let res = match operand {
            Variable::String(operand) => {
                match operand.as_str() {
                    "=" => {
                        if source.to_string() == target.to_string() {
                            0
                        } else {
                            1
                        }
                    }
                    "!=" => {
                        if source.to_string() != target.to_string() {
                            0
                        } else {
                            1
                        }
                    }
                    _ => bail!("Unsupported operand: {}", operand)
                }
            }
            _ => bail!("Unsupported operand: {}", operand)
        };
        Ok(Variable::I32(res))
    }
    map.insert("test".to_string(), NativeFunction {
        name: "test".to_string(),
        description: "Compares values. Supported operands are = != > < >= <=".to_string(),
        args: vec![String::from("source"), String::from("operand"), String::from("target")],
        func: rush_test
    });

    map
}