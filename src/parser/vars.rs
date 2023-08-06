use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use anyhow::{bail, Result};
use os_pipe::{PipeReader, PipeWriter};
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
    Array(Vec<Variable>),
    Bool(bool)
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
            Variable::Bool(val) => {
                if *val {
                    String::from("true")
                } else {
                    String::from("false")
                }
            },
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

pub fn variables_to_string(vars: Vec<Variable>) -> String {
    let mut str = String::new();
    for (i, var) in vars.iter().enumerate() {
        str += &*var.clone().to_string();
        if i < vars.len() - 1 {
            str += " ";
        }
    }
    str
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

pub struct NativeFunction {
    pub name: String,
    pub description: String,
    pub args: Vec<String>,
    pub func: fn(&mut Context, Vec<Variable>) -> Result<Variable>
}

impl Debug for NativeFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeFunction {{ name: {}, description: {} }}", self.name, self.description)
    }
}

pub enum AnyFunction<'a> {
    Native(&'a mut NativeFunction),
    UserDefined(&'a mut FunctionDefinitionExpression)
}

pub struct Overrides {
    pub stdin: Option<PipeReader>,
    pub stdout: Option<PipeWriter>,
    pub stderr: Option<PipeWriter>
}

#[derive(Debug)]
pub struct Scope {
    /// list of variables
    pub vars: HashMap<String, Variable>,
    /// list of functions
    pub func: HashMap<String, FunctionDefinitionExpression>,
    /// list of file descriptors, to be closed when the scope is left
    pub fd: Vec<usize>,
    pub stdin_override: Option<PipeReader>,
    pub stdout_override: Option<PipeWriter>,
    pub stderr_override: Option<PipeWriter>
}

#[derive(Debug)]
pub struct Context {
    pub scopes: Vec<Scope>,
    /// env variables
    pub exports: HashMap<String, Variable>,
    /// list of native functions (Rust functions)
    pub native_func: HashMap<String, NativeFunction>,
    /// number of break statements called
    pub break_num: u16,
    /// number of continue statements called
    pub continue_num: u16
}

impl Context {
    pub fn new() -> Context {
        let mut res = Context {
            scopes: Vec::new(),
            exports: HashMap::new(),
            native_func: HashMap::new(),
            break_num: 0,
            continue_num: 0
        };
        res.add_scope();
        res
    }
    pub fn pop_scope(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }
    pub fn add_scope(&mut self) {
        let scope = Scope {
            func: HashMap::new(),
            vars: HashMap::new(),
            fd: Vec::new(),
            stdin_override: None,
            stdout_override: None,
            stderr_override: None
        };
        self.scopes.push(scope);
    }

    pub fn get_var(&mut self, var: &str) -> Option<&mut Variable> {
        if var.starts_with("env::") {
            let key = var.replace("env::", "");
            return match self.exports.get_mut(&key) {
                Some(val) => {
                    return Some(val);
                },
                None => None
            }
        }
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

    pub fn get_last_exit_code(&mut self) -> Option<i32> {
        let var = self.get_var("?");
        match var {
            Some(Variable::I32(int)) => Some(*int),
            _ => None,
        }
    }

    pub fn set_var(&mut self, key: String, val: Variable) {
        let vars = &mut self.scopes.last_mut().unwrap().vars;
        if key.starts_with("env::") {
            let key = key.replace("env::", "");
            self.exports.insert(key, Variable::String(val.to_string()));
        }
        vars.insert(key, val);
    }

    pub fn get_func(&mut self, key: &str) -> Option<AnyFunction> {
        for scope in self.scopes.iter_mut().rev() {
            let funcs = &mut scope.func;
            let val = funcs.get_mut(key);
            match val {
                None => {}
                Some(val) => {
                    return Some(AnyFunction::UserDefined(val));
                }
            }
        }
        let val = self.native_func.get_mut(key);
        match val {
            None => {}
            Some(val) => {
                return Some(AnyFunction::Native(val));
            }
        }
        None
    }

    pub fn set_func(&mut self, key: String, val: FunctionDefinitionExpression) {
        let func = &mut self.scopes.last_mut().unwrap().func;
        func.insert(key, val);
    }

    /// Gets relevant overrides. Should only be used before running a command, as it will clone all pipes
    pub fn get_overrides(&self) -> Result<Overrides> {
        let mut overrides = Overrides {
            stdin: None,
            stdout: None,
            stderr: None
        };

        for scope in self.scopes.iter().rev() {
            match overrides.stdin {
                Some(_) => {}
                None => {
                    match &scope.stdin_override {
                        Some(stdin) => overrides.stdin = Some(stdin.try_clone()?),
                        None => {}
                    }
                }
            }
            match overrides.stderr {
                Some(_) => {}
                None => {
                    match &scope.stderr_override {
                        Some(stderr) => overrides.stderr = Some(stderr.try_clone()?),
                        None => {}
                    }
                }
            }
            match overrides.stdout {
                Some(_) => {}
                None => {
                    match &scope.stdout_override {
                        Some(stdout) => overrides.stdout = Some(stdout.try_clone()?),
                        None => {}
                    }
                }
            }
        }

        Ok(overrides)
    }
}
