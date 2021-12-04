use std::collections::HashMap;
use std::ops::{Add, Deref};
use crate::parser::ast::FunctionExpression;
use crate::parser::escape;

#[derive(Debug)]
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
            Variable::String(var) => String::from(var),
            Variable::I32(num) => num.to_string(),
            Variable::I64(num) => num.to_string(),
            Variable::I128(num) => num.to_string(),
            Variable::U32(num) => num.to_string(),
            Variable::U64(num) => num.to_string(),
            Variable::U128(num) => num.to_string(),
            Variable::F32(num) => num.to_string(),
            Variable::F64(num) => num.to_string(),
            Variable::HMap(map) => {
                String::from("[Object object]")
            },
            Variable::Array(vars) => {
                let mut str = String::new();
                for var in vars {
                    str += &*var.clone().to_string();
                    str += " ";
                }
                str
            }
        }
    }

    pub fn index(self: &Self) -> &Variable {
        match self {
            _ => panic!("Cannot index unsupported types")
        }
    }
}

#[derive(Debug)]
pub struct Scope {
    pub active: bool,
    pub vars: HashMap<String, Variable>,
    pub func: HashMap<String, FunctionExpression>
}

#[derive(Debug)]
pub struct Context {
    pub scopes: Vec<Scope>,
    pub exports: HashMap<String, String>
}

impl Context {
    pub fn new() -> Context {
        Context {
            scopes: Vec::new()
        }
    }
    pub fn pop_scope(self: &mut Self) -> Option<Scope> {
        self.scopes.pop()
    }
    pub fn add_scope(self: &mut Self, active: bool) {
        let scope = Scope {
            active,
            func: HashMap::new(),
            vars: HashMap::new()
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
}