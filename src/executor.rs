use std::{collections::HashMap, fmt::Debug, sync::Mutex};

use anyhow::Result;
use gc::{Gc, GcCell, Trace, Finalize};

use crate::parser::{And, Command, CommandPipe, For, FormatString, FormatStringPart, FunctionDefinition, If, Index, Loop, Not, Or, Primitive, Set, SourceFilePipe, Statement, TargetFilePipe, Value, While};

#[derive(Clone, Debug, PartialEq, Trace, Finalize)]
pub enum Type {
    Number(i64),
    String(String),
    Void,
    Heap(HeapCell)
}

impl Type {
    fn heap(t: HeapType) -> Self {
        Type::Heap(HeapCell::new(t))
    }
}

#[derive(Clone, Debug, Trace, Finalize)]
pub struct HeapCell(Gc<GcCell<HeapType>>);

impl HeapCell {
    fn new(t: HeapType) -> Self {
        Self(Gc::new(GcCell::new(t)))
    }
}

impl PartialEq for HeapCell {
    fn eq(&self, other: &Self) -> bool {
        Gc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Trace, Finalize)]
pub enum HeapType {
    Function(Function),
    Array(Vec<Type>),
    Object(HashMap<String, Type>),
}

#[derive(Default)]
pub struct Context {
    scopes: Vec<Gc<GcCell<HashMap<String, Type>>>>,
    returning: Option<Type>
}

#[derive(Debug, Trace, Finalize)]
enum Function {
    Native(NativeFunction),
    UserDefined(UserFunction)
}

#[derive(Trace, Finalize)]
struct NativeFunction {
    name: String,
    #[unsafe_ignore_trace]
    body: Mutex<Box<dyn Fn(Vec<Type>) -> Type>>
}

#[derive(Trace, Finalize)]
struct UserFunction {
    #[unsafe_ignore_trace]
    def: Box<FunctionDefinition>,
    /// captured scopes to allow referencing variables in callbacks
    scopes: Vec<Gc<GcCell<HashMap<String, Type>>>>,
}

impl Debug for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeFunction")
            .field("name", &self.name)
            .finish()
    }
}

impl Debug for UserFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.def.fmt(f)
    }
}

trait GetValue {
    fn get(&mut self, context: &mut Context) -> Result<Type>;
}

trait Exec {}

impl GetValue for Value {
    fn get(&mut self, context: &mut Context) -> Result<Type> {
        match self {
            Value::Primitive(p) => p.get(context),
            _ => Err(anyhow::anyhow!("Not implemented yet"))
        }
    }
}
impl GetValue for Primitive {
    fn get(&mut self, context: &mut Context) -> Result<Type> {
        match self {
            Primitive::Number(n) => Ok(Type::Number(*n)),
            Primitive::FormatString(s) => s.get(context),
            Primitive::Index(i) => i.get(context),
        }
    }
}
impl GetValue for FormatString {}
impl GetValue for FormatStringPart {}
impl GetValue for Index {}

impl Exec for Statement {}
impl Exec for Command {}
impl Exec for Set {}
impl Exec for For {}
impl Exec for While {}
impl Exec for If {}
impl Exec for Loop {}
impl Exec for Or {}
impl Exec for And {}
impl Exec for Not {}
impl Exec for CommandPipe {}
impl Exec for TargetFilePipe {}
impl Exec for SourceFilePipe {}
impl Exec for FunctionDefinition {}
impl Exec for UserFunction {}
impl Exec for NativeFunction {}