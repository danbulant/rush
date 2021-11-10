use std::collections::HashMap;

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
    hmap(HashMap<String, Variable>),
    array(Vec<Variable>)
}

#[derive(Debug)]
pub struct Scope {
    active: bool,
    vars: HashMap<String, Variable>
}

#[derive(Debug)]
pub struct Context {
    pub scopes: Vec<Scope>,
    pub parent_context: Option<Box<Context>>
}

impl Context {
    pub fn new() -> Context {
        Context {
            scopes: Vec::new(),
            parent_context: None
        }
    }
    pub fn add_scope(self: &mut Context, active: bool) {
        let scope = Scope {
            active: active,
            vars: HashMap::new()
        };
        self.scopes.push(scope);
    }
}