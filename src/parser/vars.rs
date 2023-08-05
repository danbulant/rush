use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use anyhow::{bail, Result};
use crate::parser::ast::FunctionDefinitionExpression;

#[derive(Debug, Clone)]
pub enum Variable {
    String(String),
    I32(i32),
    I64(i64),
    I128(i128),
    U32(u32),
    U64(u64),
    U128(u128),
    F32(f32),
    F64(f64),
    HMap(HashMap<String, Variable>),
    Array(Vec<Variable>)
}

impl Display for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Variable::String(var) => {
                var.clone()
            },
            Variable::I32(num) => num.to_string(),
            Variable::I64(num) => num.to_string(),
            Variable::I128(num) => num.to_string(),
            Variable::U32(num) => num.to_string(),
            Variable::U64(num) => num.to_string(),
            Variable::U128(num) => num.to_string(),
            Variable::F32(num) => num.to_string(),
            Variable::F64(num) => num.to_string(),
            Variable::HMap(_map) => {
                String::from("[Object object]")
            },
            Variable::Array(vars) => {
                let len = vars.len();
                if len == 1 {
                    return match vars.get(0) {
                        Some(var) => write!(f, "{}", var),
                        None => write!(f, "[]")
                    }
                }
                let mut str = String::new();
                for (i, var) in vars.iter().enumerate() {
                    str += &*var.clone().to_string();
                    if i < len - 1 {
                        str += " ";
                    }
                }
                str
            }
        })
    }
}

impl Variable {
    pub fn index(&self, index: &Variable) -> Result<&Variable> {
        match self {
            Variable::HMap(map) => {
                match index {
                    Variable::String(key) => {
                        match map.get(key) {
                            Some(val) => Ok(val),
                            None => bail!("Key not found")
                        }
                    }
                    _ => bail!("Cannot index with non-string")
                }
            },
            Variable::Array(arr) => {
                match index {
                    Variable::I32(idx) => {
                        match arr.get(*idx as usize) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    Variable::I64(idx) => {
                        match arr.get(*idx as usize) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    Variable::I128(idx) => {
                        match arr.get(*idx as usize) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    Variable::F32(idx) => {
                        match arr.get(*idx as usize) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    Variable::F64(idx) => {
                        match arr.get(*idx as usize) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    Variable::U32(idx) => {
                        match arr.get(*idx as usize) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    Variable::U64(idx) => {
                        match arr.get(*idx as usize) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    Variable::U128(idx) => {
                        match arr.get(*idx as usize) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    Variable::String(idx) => {
                        match arr.get(idx.parse::<usize>()?) {
                            Some(val) => Ok(val),
                            None => bail!("Index out of bounds")
                        }
                    }
                    _ => bail!("Cannot index with non-integer")
                }
            },
            _ => bail!("Cannot index unsupported types")
        }
    }
}

#[derive(Debug)]
pub struct Scope {
    /// list of variables
    pub vars: HashMap<String, Variable>,
    /// list of functions
    pub func: HashMap<String, FunctionDefinitionExpression>,
    /// list of file descriptors, to be closed when the scope is left
    pub fd: Vec<usize>
}

#[derive(Debug)]
pub struct Context {
    pub scopes: Vec<Scope>,
    pub exports: HashMap<String, String>,
    pub break_num: u16,
    pub continue_num: u16
}

impl Context {
    pub fn new() -> Context {
        let mut res = Context {
            scopes: Vec::new(),
            exports: HashMap::new(),
            break_num: 0,
            continue_num: 0
        };
        res.add_scope();
        res
    }
    pub fn pop_scope(self: &mut Self) -> Option<Scope> {
        self.scopes.pop()
    }
    pub fn add_scope(self: &mut Self) {
        let scope = Scope {
            func: HashMap::new(),
            vars: HashMap::new(),
            fd: Vec::new()
        };
        self.scopes.push(scope);
    }

    pub fn get_var(self: &mut Self, var: &str) -> Option<&mut Variable> {
        for scope in self.scopes.iter_mut().rev() {
            let vars = &mut scope.vars;
            let val = vars.get_mut(var);
            match val {
                None => {}
                Some(val) => {
                    return Some(val);
                }
            }
        }
        None
    }

    pub fn set_var(&mut self, key: String, val: Variable) {
        let vars = &mut self.scopes.last_mut().unwrap().vars;
        vars.insert(key, val);
    }

    pub fn get_func(self: &mut Self, key: &str) -> Option<&mut FunctionDefinitionExpression> {
        for scope in self.scopes.iter_mut().rev() {
            let funcs = &mut scope.func;
            let val = funcs.get_mut(key);
            match val {
                None => {}
                Some(val) => {
                    return Some(val);
                }
            }
        }
        None
    }

    pub fn set_func(&mut self, key: String, val: FunctionDefinitionExpression) {
        let mut func = &mut self.scopes.last_mut().unwrap().func;
        func.insert(key, val);
    }
}
