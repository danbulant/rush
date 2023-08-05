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

    fn rush_true(_ctx: &mut Context, _args: Vec<Variable>) -> Result<Variable> {
        Ok(Variable::I32(0))
    }
    map.insert("true".to_string(), NativeFunction {
        name: "true".to_string(),
        description: "Returns 0".to_string(),
        args: vec![],
        func: rush_true
    });

    fn rush_false(_ctx: &mut Context, _args: Vec<Variable>) -> Result<Variable> {
        Ok(Variable::I32(1))
    }
    map.insert("false".to_string(), NativeFunction {
        name: "false".to_string(),
        description: "Returns 1".to_string(),
        args: vec![],
        func: rush_false
    });

    fn rush_export(ctx: &mut Context, args: Vec<Variable>) -> Result<Variable> {
        if args.len() != 1 && args.len() != 3 {
            bail!("Expected 1 (name) or 3 (name = value) arguments, got {}", args.len());
        }
        let name = args.get(0).unwrap();
        if args.len() == 1 {
            let value = ctx.get_var(&name.to_string());
            match value {
                Some(value) => {
                    let val = value.clone();
                    ctx.exports.insert(name.to_string(), val);
                }
                None => return Ok(Variable::I32(1))
            }
        } else {
            let value = args.get(2).unwrap();
            ctx.set_var(name.to_string(), value.clone());
        }
        Ok(Variable::I32(0))
    }
    map.insert("export".to_string(), NativeFunction {
        name: "export".to_string(),
        description: "Exports a variable to the environment".to_string(),
        args: vec![String::from("name"), String::from("="), String::from("value")],
        func: rush_export
    });

    fn rush_typeof(_ctx: &mut Context, args: Vec<Variable>) -> Result<Variable> {
        if args.len() != 1 {
            bail!("Expected 1 argument, got {}", args.len());
        }
        let arg = args.get(0).unwrap();
        let res = match arg {
            Variable::String(_) => "string",
            Variable::I32(_) => "i32",
            Variable::I64(_) => "i64",
            Variable::I128(_) => "i128",
            Variable::U32(_) => "u32",
            Variable::U64(_) => "u64",
            Variable::U128(_) => "u128",
            Variable::F32(_) => "f32",
            Variable::F64(_) => "f64",
            Variable::Bool(_) => "bool",
            Variable::Array(_) => "array",
            Variable::HMap(_) => "HMap"
        };
        Ok(Variable::String(res.to_string()))
    }
    map.insert("typeof".to_string(), NativeFunction {
        name: "typeof".to_string(),
        description: "Returns the type of a variable".to_string(),
        args: vec![String::from("var")],
        func: rush_typeof
    });

    map
}