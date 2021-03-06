use std::collections::HashMap;
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

impl Variable {
    pub fn to_string(self: &Self) -> String {
        match self {
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
                if len == 1 { return vars.get(0).unwrap().to_string(); }
                let mut str = String::new();
                let mut i = 0;
                for var in vars {
                    str += &*var.clone().to_string();
                    if i < len - 1 {
                        str += " ";
                    }
                    i += 1;
                }
                str
            }
        }
    }

    pub fn index(self: &Self, index: &Variable) -> Result<&Variable> {
        match self {
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
        for mut scope in self.scopes.iter_mut().rev() {
            let mut vars = &mut scope.vars;
            let val = vars.get_mut(var);
            match val {
                None => {},
                Some(val) => {
                    return Some(val);
                }
            }
        }
        None
    }

    pub fn set_var(&mut self, key: String, val: Variable) {
        let mut vars = &mut self.scopes.last_mut().unwrap().vars;
        vars.insert(key, val);
    }

    pub fn get_func(self: &mut Self, key: &str) -> Option<&mut FunctionDefinitionExpression> {
        for mut scope in self.scopes.iter_mut().rev() {
            let mut funcs = &mut scope.func;
            let val = funcs.get_mut(key);
            match val {
                None => {},
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
