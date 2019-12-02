#![deny(missing_debug_implementations)]
#![allow(dead_code, clippy::use_self, clippy::module_name_repetitions)]
// todo: heuristic for byte and friends being converted to int (e.g. indexing into array)
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};

use crate::attributes::Attribute;
use crate::builder::ClassFileBuilder;
use crate::code::Instruction;
pub use crate::common::Type;
use crate::errors::{JResult, ParseError};
pub use crate::fields::{FieldAccessFlags, FieldDescriptor, FieldInfo};
use crate::methods::{MethodDescriptor, MethodInfo};
pub use crate::pool::PoolKind;
pub use crate::version::MajorVersion;

const TEST_CLASS_FILE_PATH: &str = "test.class";

pub mod attributes;
mod builder;
pub mod code;
mod common;
pub mod errors;
mod fields;
pub mod methods;
mod pool;
mod version;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ClassAccessFlags {
    is_public: bool,
    is_final: bool,
    is_super: bool,
    is_interface: bool,
    is_abstract: bool,
    is_synthetic: bool,
    is_annotation: bool,
    is_enum: bool,
}

impl From<u16> for ClassAccessFlags {
    fn from(n: u16) -> Self {
        ClassAccessFlags::from_u16(n)
    }
}

impl ClassAccessFlags {
    const PUBLIC: u16 = 0x0001;
    const FINAL: u16 = 0x0010;
    const SUPER: u16 = 0x0020;
    const INTERFACE: u16 = 0x0200;
    const ABSTRACT: u16 = 0x0400;
    const SYNTHETIC: u16 = 0x1000;
    const ANNOTATION: u16 = 0x2000;
    const ENUM: u16 = 0x4000;

    #[must_use]
    pub const fn from_u16(n: u16) -> ClassAccessFlags {
        ClassAccessFlags {
            is_public: (n & ClassAccessFlags::PUBLIC) != 0,
            is_final: (n & ClassAccessFlags::FINAL) != 0,
            is_super: (n & ClassAccessFlags::SUPER) != 0,
            is_interface: (n & ClassAccessFlags::INTERFACE) != 0,
            is_abstract: (n & ClassAccessFlags::ABSTRACT) != 0,
            is_synthetic: (n & ClassAccessFlags::SYNTHETIC) != 0,
            is_annotation: (n & ClassAccessFlags::ANNOTATION) != 0,
            is_enum: (n & ClassAccessFlags::ENUM) != 0,
        }
    }
}

impl std::fmt::Display for ClassAccessFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{}-{}-{}-{}-{}-{}-{}",
            if self.is_public { "PUBLIC" } else { "" },
            if self.is_final { "FINAL" } else { "" },
            if self.is_super { "SUPER" } else { "" },
            if self.is_interface { "INTERFACE" } else { "" },
            if self.is_abstract { "ABSTRACT" } else { "" },
            if self.is_synthetic { "SYNTHETIC" } else { "" },
            if self.is_annotation { "ANNOTATION" } else { "" },
            if self.is_enum { "ENUM" } else { "" },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassFile {
    pub(crate) version: (MajorVersion, u16),
    pub(crate) const_pool: Vec<PoolKind>,
    pub(crate) access_flags: ClassAccessFlags,
    pub(crate) this_class: u16,
    pub(crate) super_class: u16,
    pub(crate) interfaces: Vec<u16>,
    pub(crate) fields: Vec<FieldInfo>,
    pub(crate) methods: Vec<MethodInfo>,
    pub(crate) attributes: Vec<Attribute>,
}

/// Methods for creating `ClassFiles`s
impl ClassFile {
    pub fn from_bufreader<R: Read + BufRead>(reader: R) -> JResult<ClassFile> {
        ClassFileBuilder {
            reader,
            const_pool: Vec::new(),
        }
        .parse()
    }

    pub fn from_path<P: AsRef<std::path::Path>>(p: P) -> JResult<ClassFile> {
        let buffer = BufReader::new(File::open(p)?);
        ClassFile::from_bufreader(buffer)
    }
}

impl ClassFile {
    #[must_use]
    pub const fn version(&self) -> (MajorVersion, u16) {
        self.version
    }

    #[must_use]
    pub const fn const_pool(&self) -> &Vec<PoolKind> {
        &self.const_pool
    }

    #[must_use]
    pub const fn access_flags(&self) -> ClassAccessFlags {
        self.access_flags
    }

    pub fn class_name(&self) -> JResult<&str> {
        match self.const_pool[usize::from(self.this_class - 1)] {
            PoolKind::Class(name_index) => match self.const_pool[usize::from(name_index - 1)] {
                PoolKind::Utf8(ref s) => Ok(s),
                _ => Err(ParseError::IndexError(line!())),
            },
            _ => Err(ParseError::IndexError(line!())),
        }
    }

    pub fn super_class_name(&self) -> JResult<&str> {
        match self.const_pool[usize::from(self.super_class - 1)] {
            PoolKind::Class(name_index) => match self.const_pool[usize::from(name_index - 1)] {
                PoolKind::Utf8(ref s) => Ok(s),
                _ => Err(ParseError::IndexError(line!())),
            },
            _ => Err(ParseError::IndexError(line!())),
        }
    }

    pub fn interfaces(&self) -> JResult<Vec<&String>> {
        let mut interfaces = Vec::new();
        for interface_idx in &self.interfaces {
            interfaces.push(match self.const_pool[usize::from(interface_idx - 1)] {
                PoolKind::Utf8(ref s) => s,
                _ => unimplemented!(),
            });
        }
        Ok(interfaces)
    }

    #[must_use]
    pub const fn fields(&self) -> &Vec<FieldInfo> {
        &self.fields
    }

    pub fn field_names(&self) -> JResult<Vec<&str>> {
        let mut fields = Vec::new();
        for field in &self.fields {
            fields.push(match self.const_pool[usize::from(field.name_index - 1)] {
                PoolKind::Utf8(ref s) => s.as_str(),
                _ => unimplemented!(),
            });
        }
        Ok(fields)
    }

    pub fn field_types(&self) -> JResult<Vec<&str>> {
        let mut fields = Vec::new();
        for field in &self.fields {
            fields.push(
                match self.const_pool[usize::from(field.descriptor_index - 1)] {
                    PoolKind::Utf8(ref s) => s.as_str(),
                    _ => unimplemented!(),
                },
            );
        }
        Ok(fields)
    }

    pub fn field_names_and_types(&self) -> JResult<Vec<(&str, &str)>> {
        Ok(self
            .field_names()?
            .into_iter()
            .zip(self.field_types()?)
            .collect())
    }

    #[must_use]
    pub const fn methods(&self) -> &Vec<MethodInfo> {
        &self.methods
    }

    #[must_use]
    pub fn method_names(&self) -> Vec<&String> {
        self.methods.iter().map(|m| &m.name).collect()
    }

    pub fn method_by_name<T: AsRef<str>>(&self, name: T) -> JResult<&MethodInfo> {
        for method in &self.methods {
            if method.name == name.as_ref() {
                return Ok(method);
            }
        }
        Err(ParseError::MethodNotFound)
    }

    #[must_use]
    pub fn methods_name_hash(&self) -> HashMap<&str, &MethodInfo> {
        let names = self.method_names();
        let mut hash = HashMap::new();
        for (ii, name) in names.iter().enumerate() {
            hash.insert(name.as_str(), &self.methods[ii]);
        }
        hash
    }

    #[must_use]
    pub const fn attributes(&self) -> &Vec<Attribute> {
        &self.attributes
    }

    #[must_use]
    pub fn source_file(&self) -> Option<&str> {
        for attr in &self.attributes {
            match attr {
                Attribute::SourceFile(idx) => match self.const_pool[usize::from(idx - 1)] {
                    PoolKind::Utf8(ref s) => return Some(s.as_str()),
                    _ => unimplemented!(),
                },
                _ => continue,
            }
        }
        None
    }

    pub fn class_name_from_index(&self, index: u16) -> JResult<String> {
        if let PoolKind::Class(i) = &self.const_pool[usize::from(index - 1)] {
            Ok(self.utf_from_index(*i)?)
        } else {
            Err(ParseError::IndexError(line!()))
        }
    }

    pub fn utf_from_index(&self, index: u16) -> JResult<String> {
        if let PoolKind::Utf8(s) = &self.const_pool[usize::from(index - 1)] {
            Ok(s.clone())
        } else {
            Err(ParseError::IndexError(line!()))
        }
    }

    pub fn read_methodref_from_index(
        &self,
        index: u16,
    ) -> JResult<(String, String, MethodDescriptor)> {
        match &self.const_pool[usize::from(index - 1)] {
            PoolKind::MethodRef {
                name_and_type_index,
                class_index,
            } => {
                let (name, sig): (String, MethodDescriptor) =
                    match &self.const_pool[usize::from(name_and_type_index - 1)] {
                        PoolKind::NameAndType {
                            name_index,
                            descriptor_index,
                        } => {
                            let name = self.utf_from_index(*name_index)?;
                            let ty = MethodDescriptor::new(self.utf_from_index(*descriptor_index)?);
                            (name, ty)
                        }
                        _ => return Err(ParseError::IndexError(line!())),
                    };
                let class = self
                    .class_name_from_index(*class_index)?
                    .split('/')
                    .last()
                    .unwrap()
                    .to_owned();
                Ok((class, name, sig))
            }
            _ => Err(ParseError::IndexError(line!())),
        }
    }

    pub fn read_fieldref_from_index(&self, index: u16) -> JResult<(String, String, Type)> {
        match &self.const_pool[usize::from(index - 1)] {
            PoolKind::FieldRef {
                name_and_type_index,
                class_index,
            } => {
                let (name, sig): (String, Type) = match &self.const_pool
                    [usize::from(name_and_type_index - 1)]
                {
                    PoolKind::NameAndType {
                        name_index,
                        descriptor_index,
                    } => {
                        let name = self.utf_from_index(*name_index)?;
                        let ty = FieldDescriptor::new(self.utf_from_index(*descriptor_index)?).ty;
                        (name, ty)
                    }
                    _ => return Err(ParseError::IndexError(line!())),
                };
                let class = self
                    .class_name_from_index(*class_index)?
                    .split('/')
                    .last()
                    .unwrap()
                    .to_owned();
                Ok((class, name, sig))
            }
            _ => Err(ParseError::IndexError(line!())),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum Comparison {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqualThan,
    LessEqualThan,
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Comparison::Equal => write!(f, "=="),
            Comparison::NotEqual => write!(f, "!="),
            Comparison::GreaterThan => write!(f, ">"),
            Comparison::LessThan => write!(f, "<"),
            Comparison::GreaterEqualThan => write!(f, ">="),
            Comparison::LessEqualThan => write!(f, "<="),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum StackEntry {
    Int(i32),
    Float(f32),
    Double(f64),
    Long(i64),
    Array(Type, usize, Vec<StackEntry>),
    New(Box<StackEntry>),
    Index(Box<StackEntry>, Box<StackEntry>, Type),
    Class(String),
    Cast(Type, Box<StackEntry>),
    Add(Box<StackEntry>, Box<StackEntry>),
    Sub(Box<StackEntry>, Box<StackEntry>),
    Mul(Box<StackEntry>, Box<StackEntry>),
    Div(Box<StackEntry>, Box<StackEntry>),
    Ident(String, Type),
    If(Box<StackEntry>, Comparison, Box<StackEntry>),
    Function(String, Vec<StackEntry>, Type),
    Field(Type, String),
    String(String),
    Unitialized,
}

impl fmt::Display for StackEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackEntry::Int(a) => write!(f, "{}", a),
            StackEntry::Long(a) => write!(f, "{}l", a),
            StackEntry::Float(a) => write!(f, "{}f", a),
            StackEntry::Double(a) => write!(f, "{}d", a),
            StackEntry::Array(_, _, els) => write!(
                f,
                "{{ {} }}",
                els.iter()
                    .rev()
                    .map(|a| format!("{}", a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            StackEntry::New(val) => write!(f, "new {}", val),
            StackEntry::Index(arr, idx, _ty) => write!(f, "{}[{}]", arr, idx),
            StackEntry::Class(name) => write!(f, "{}", name),
            StackEntry::Cast(ty, val) => write!(f, "({}) {}", ty, val),
            StackEntry::Add(a, b) => write!(f, "({} + {})", b, a),
            StackEntry::Sub(a, b) => write!(f, "({} - {})", b, a),
            StackEntry::Mul(a, b) => write!(f, "{} * {}", b, a),
            StackEntry::Div(a, b) => write!(f, "{} / {}", b, a),
            StackEntry::Ident(s, _ty) => write!(f, "{}", s),
            StackEntry::String(s) => write!(f, "\"{}\"", s),
            StackEntry::If(if_, cmp, else_) => writeln!(f, "if ({} {} {}) {{", if_, cmp, else_),
            StackEntry::Function(name, args, _) => write!(
                f,
                "{}({})",
                name,
                args.iter()
                    .rev()
                    .map(|a| format!("{}", a))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            StackEntry::Field(_, name) => write!(f, "{}", name),
            StackEntry::Unitialized => panic!("attempted to render unitialized entry"),
        }
    }
}

#[derive(Debug)]
struct Codegen<W: Write> {
    class: ClassFile,
    buf: W,
    stack: Vec<StackEntry>,
    local_variables: HashMap<usize, StackEntry>,
    tokens: std::vec::IntoIter<Instruction>,
}

impl<W: Write> Codegen<W> {
    fn codegen(mut self) -> JResult<()> {
        while let Some(tok) = self.tokens.next() {
            // dbg!(tok);
            match tok {
                Instruction::BiPush(n) => self.stack.push(StackEntry::Int(i32::from(n))),
                Instruction::SIPush(n) => self.stack.push(StackEntry::Int(i32::from(n))),
                Instruction::Ldc(n) | Instruction::LdcW(n) => {
                    let val = &self.class.const_pool[usize::from(n - 1)];
                    match val {
                        PoolKind::String(idx) => self
                            .stack
                            .push(StackEntry::String(self.class.utf_from_index(*idx)?)),
                        PoolKind::Integer(i) => self.stack.push(StackEntry::Int(*i as i32)),
                        PoolKind::Float { bytes } => {
                            self.stack.push(StackEntry::Float(match bytes {
                                0x7f80_0000 => std::f32::INFINITY,
                                0xff80_0000 => std::f32::NEG_INFINITY,
                                0x7f80_0001..=0x7fff_ffff | 0xff80_0001..=0xffff_ffff => {
                                    std::f32::NAN
                                }
                                _ => f32::from_be_bytes(bytes.to_be_bytes()),
                            }))
                        }
                        _ => unimplemented!(),
                    }
                }
                Instruction::Ldc2W(n) => {
                    let val = &self.class.const_pool[usize::from(n - 1)];
                    match val {
                        PoolKind::Double {
                            high_bytes,
                            low_bytes,
                        } => {
                            let bits: u64 = (u64::from(*high_bytes) << 32) + u64::from(*low_bytes);
                            self.stack.push(StackEntry::Double(match bits {
                                0x7ff0_0000_0000_0000 => std::f64::INFINITY,
                                0xfff0_0000_0000_0000 => std::f64::NEG_INFINITY,
                                0x7ff0_0000_0000_0001..=0x7fff_ffff_ffff_ffff
                                | 0xfff0_0000_0000_0001..=0xffff_ffff_ffff_ffff => std::f64::NAN,
                                _ => {
                                    let a = high_bytes.to_be_bytes();
                                    let b = low_bytes.to_be_bytes();
                                    f64::from_be_bytes([
                                        a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3],
                                    ])
                                }
                            }));
                        }
                        PoolKind::Long(long) => self.stack.push(StackEntry::Long(*long)),
                        _ => unimplemented!("Ldc2w should only encounter doubles and longs"),
                    }
                }

                Instruction::IConstM1 => self.stack.push(StackEntry::Int(-1)),
                Instruction::IConst0 => self.stack.push(StackEntry::Int(0)),
                Instruction::IConst1 => self.stack.push(StackEntry::Int(1)),
                Instruction::IConst2 => self.stack.push(StackEntry::Int(2)),
                Instruction::IConst3 => self.stack.push(StackEntry::Int(3)),
                Instruction::IConst4 => self.stack.push(StackEntry::Int(4)),
                Instruction::IConst5 => self.stack.push(StackEntry::Int(5)),
                Instruction::FConst0 => self.stack.push(StackEntry::Float(0.0)),
                Instruction::FConst1 => self.stack.push(StackEntry::Float(1.0)),
                Instruction::FConst2 => self.stack.push(StackEntry::Float(2.0)),
                Instruction::DConst0 => self.stack.push(StackEntry::Double(0.0)),
                Instruction::DConst1 => self.stack.push(StackEntry::Double(1.0)),
                Instruction::LConst0 => self.stack.push(StackEntry::Long(0)),
                Instruction::LConst1 => self.stack.push(StackEntry::Long(1)),

                Instruction::Aload0
                | Instruction::Fload0
                | Instruction::Dload0
                | Instruction::ILoad0
                | Instruction::LLoad0 => self.load(0),
                Instruction::Aload1
                | Instruction::Fload1
                | Instruction::Dload1
                | Instruction::ILoad1
                | Instruction::LLoad1 => self.load(1),
                Instruction::Aload2
                | Instruction::Fload2
                | Instruction::Dload2
                | Instruction::ILoad2
                | Instruction::LLoad2 => self.load(2),
                Instruction::ALoad3
                | Instruction::Fload3
                | Instruction::Dload3
                | Instruction::ILoad3
                | Instruction::LLoad3 => self.load(3),
                Instruction::AALoad
                | Instruction::BALoad
                | Instruction::CALoad
                | Instruction::DALoad
                | Instruction::FALoad
                | Instruction::IALoad
                | Instruction::LALoad
                | Instruction::SALoad => {
                    let index = self
                        .stack
                        .pop()
                        .expect("expected local variable at index to exist");
                    let array = self
                        .stack
                        .pop()
                        .expect("expected local variable at index to exist");
                    let ty = match &array {
                        StackEntry::Array(ty, _, _) => ty,
                        StackEntry::Index(_, _, ty) => ty,
                        StackEntry::Ident(_, ty) => ty,
                        _ => unimplemented!(),
                    }
                    .clone();
                    let ty = match ty {
                        Type::Reference(r) => *r,
                        _ => ty,
                    };
                    self.stack
                        .push(StackEntry::Index(Box::new(array), Box::new(index), ty))
                }

                Instruction::IStore(n) => self.istore(n)?,
                Instruction::IStore1 => self.istore(1)?,
                Instruction::IStore2 => self.istore(2)?,
                Instruction::IStore3 => self.istore(3)?,
                Instruction::FStore(n) => self.fstore(n)?,
                Instruction::FStore1 => self.fstore(1)?,
                Instruction::FStore2 => self.fstore(2)?,
                Instruction::FStore3 => self.fstore(3)?,
                Instruction::DStore(n) => self.dstore(n)?,
                Instruction::DStore1 => self.dstore(1)?,
                Instruction::DStore2 => self.dstore(2)?,
                Instruction::DStore3 => self.dstore(3)?,
                Instruction::LStore(n) => self.lstore(n)?,
                Instruction::LStore1 => self.lstore(1)?,
                Instruction::LStore2 => self.lstore(2)?,
                Instruction::LStore3 => self.lstore(3)?,
                Instruction::AStore(n) => self.astore(n)?,
                Instruction::AStore1 => self.astore(1)?,
                Instruction::AStore2 => self.astore(2)?,
                Instruction::AStore3 => self.astore(3)?,
                Instruction::AAStore
                | Instruction::BAStore
                | Instruction::CAStore
                | Instruction::DAStore
                | Instruction::FAStore
                | Instruction::IAStore
                | Instruction::LAStore
                | Instruction::SAStore => {
                    let val = self.stack.pop().unwrap();
                    let index: usize = match self.stack.pop().unwrap() {
                        StackEntry::Int(i) => i.try_into()?,
                        _ => unimplemented!("non-int array index"),
                    };
                    let array = self.stack.pop().unwrap();
                    match array {
                        StackEntry::Array(ty, count, mut els) => {
                            els.push(val);
                            els.swap_remove(index);
                            self.stack.push(StackEntry::Array(ty, count, els));
                        }
                        StackEntry::Ident(s, _) => {
                            writeln!(self.buf, "{}[{}] = {};", s, index, val)?;
                        }
                        _ => unimplemented!(),
                    }
                }

                Instruction::IAdd | Instruction::FAdd | Instruction::DAdd | Instruction::LAdd => {
                    let val1 = self.stack.pop().unwrap();
                    let val2 = self.stack.pop().unwrap();
                    self.stack
                        .push(StackEntry::Add(Box::new(val1), Box::new(val2)));
                }
                Instruction::Isub | Instruction::Fsub | Instruction::Dsub | Instruction::Lsub => {
                    let val1 = self.stack.pop().unwrap();
                    let val2 = self.stack.pop().unwrap();
                    self.stack
                        .push(StackEntry::Sub(Box::new(val1), Box::new(val2)));
                }
                Instruction::Imul | Instruction::Fmul | Instruction::Dmul | Instruction::Lmul => {
                    let val1 = self.stack.pop().unwrap();
                    let val2 = self.stack.pop().unwrap();
                    self.stack
                        .push(StackEntry::Mul(Box::new(val1), Box::new(val2)));
                }
                Instruction::Idiv | Instruction::Fdiv | Instruction::Ddiv | Instruction::Ldiv => {
                    let val1 = self.stack.pop().unwrap();
                    let val2 = self.stack.pop().unwrap();
                    self.stack
                        .push(StackEntry::Div(Box::new(val1), Box::new(val2)));
                }

                Instruction::Return => break,
                Instruction::AReturn
                | Instruction::Ireturn
                | Instruction::Freturn
                | Instruction::Dreturn
                | Instruction::Lreturn => {
                    writeln!(self.buf, "return {};", self.stack.pop().unwrap())?
                }

                Instruction::InvokeStatic(index) => {
                    let (class, name, descriptor) = self.class.read_methodref_from_index(index)?;
                    let mut args: Vec<StackEntry> = Vec::new();
                    for _ in 0..descriptor.args.len() {
                        args.push(self.stack.pop().unwrap());
                    }
                    let f = StackEntry::Function(
                        format!("{}.{}", class, name),
                        args,
                        descriptor.return_type.clone(),
                    );
                    if descriptor.return_type == Type::Void {
                        writeln!(self.buf, "{};", f.clone())?;
                    }
                    self.stack.push(f);
                }

                Instruction::InvokeVirtual(index) => {
                    let (_, name, descriptor) = self.class.read_methodref_from_index(index)?;
                    let mut args: Vec<StackEntry> = Vec::new();
                    for _ in 0..descriptor.args.len() {
                        args.push(self.stack.pop().expect("expected value to be on stack"));
                    }
                    let object = self.stack.pop().expect("expected value to be on stack");
                    let f = StackEntry::Function(
                        format!("{}.{}", object, name),
                        args,
                        descriptor.return_type.clone(),
                    );
                    if descriptor.return_type == Type::Void {
                        writeln!(self.buf, "{};", f.clone())?;
                    }
                    self.stack.push(f);
                }

                Instruction::GetStatic(index) => {
                    let (class, name, ty) = self.class.read_fieldref_from_index(index)?;
                    self.stack
                        .push(StackEntry::Field(ty, format!("{}.{}", &class, name)));
                }
                Instruction::GetField(index) => {
                    let (class, name, _) = self.class.read_fieldref_from_index(index)?;
                    let obj = self.stack.pop().expect("expected object on stack");
                    match obj {
                        StackEntry::Ident(s, _) => {
                            self.stack.push(StackEntry::Field(
                                Type::ClassName(class),
                                format!("{}.{}", s, name),
                            ));
                        }
                        _ => unimplemented!("non-ident field access"),
                    }
                }

                Instruction::NewArray(ty) => {
                    let ty = match ty {
                        4 => Type::Boolean, //int
                        5 => Type::Char,    //int
                        6 => Type::Float,   //float
                        7 => Type::Double,  //double
                        8 => Type::Byte,    //int
                        9 => Type::Short,   //int
                        10 => Type::Int,    //int
                        11 => Type::Long,   //long
                        _ => unimplemented!("unexpected NewArray type"),
                    };
                    let count: usize = match self.stack.pop().unwrap() {
                        StackEntry::Int(i) => i,
                        _ => unimplemented!("NewArray count is non-integer value"),
                    }
                    .try_into()?;
                    let v = vec![StackEntry::Unitialized; count];
                    self.stack.push(StackEntry::Array(ty, count, v))
                }
                Instruction::ANewArray(index) => {
                    let ty = FieldDescriptor::new(self.class.class_name_from_index(index)?).ty;
                    let count: usize = match self.stack.pop().unwrap() {
                        StackEntry::Int(i) => i,
                        _ => unimplemented!("ANewArray count is non-integer value"),
                    }
                    .try_into()?;
                    let v = vec![StackEntry::Unitialized; count];
                    self.stack.push(StackEntry::Array(ty, count, v))
                }

                Instruction::Nop => {}
                Instruction::Pop => writeln!(self.buf, "{};", self.stack.pop().unwrap())?,
                Instruction::Pop2 => {
                    let val1 = self.stack.pop().unwrap();
                    match val1 {
                        StackEntry::Long(_) | StackEntry::Double(_) => {}
                        StackEntry::Function(_, _, Type::Double)
                        | StackEntry::Function(_, _, Type::Long) => {}
                        _ => writeln!(self.buf, "{};", self.stack.pop().unwrap())?,
                    }
                    writeln!(self.buf, "{};", val1)?
                }

                Instruction::IfIcmpne(branchbyte1, branchbyte2) => {
                    let _offset: u32 = u32::from(branchbyte1) << 8 | u32::from(branchbyte2);
                    let left = self.stack.pop().unwrap();
                    let right = self.stack.pop().unwrap();
                    write!(
                        self.buf,
                        "{}",
                        StackEntry::If(Box::new(left), Comparison::NotEqual, Box::new(right))
                    )?;
                }

                Instruction::Goto(branchbyte1, branchbyte2) => {
                    let _offset: u32 = u32::from(branchbyte1) << 8 | u32::from(branchbyte2);
                    unimplemented!()
                }

                Instruction::I2b => self.cast(Type::Byte),
                Instruction::I2c => self.cast(Type::Char),
                Instruction::I2d | Instruction::F2d | Instruction::L2d => self.cast(Type::Double),
                Instruction::I2l | Instruction::F2l | Instruction::D2l => self.cast(Type::Long),
                Instruction::I2s => self.cast(Type::Short),
                Instruction::F2i | Instruction::D2i | Instruction::L2i => self.cast(Type::Int),
                Instruction::I2f | Instruction::D2f | Instruction::L2f => self.cast(Type::Float),

                Instruction::Dup => {
                    // todo: figure out initialization with `dup`
                    let val = self.stack.pop().unwrap();
                    match val {
                        StackEntry::Array(..) => self.stack.push(val),
                        _ => {
                            self.stack.push(val.clone());
                            self.stack.push(val);
                        }
                    };
                }
                Instruction::DupX1 => {
                    let val1 = self.stack.pop().unwrap();
                    let val2 = self.stack.pop().unwrap();
                    self.stack.push(val1.clone());
                    self.stack.push(val2);
                    self.stack.push(val1);
                }
                Instruction::Dup2 => {
                    let val1 = self.stack.pop().unwrap();
                    let val2 = self.stack.pop().unwrap();
                    self.stack.push(val2.clone());
                    self.stack.push(val1.clone());
                    self.stack.push(val2);
                    self.stack.push(val1);
                }
                _ => unimplemented!("instruction not yet implemented"),
            };
        }
        // buf.write_all(b"}\n}\n")?;
        Ok(())
    }

    fn astore(&mut self, idx: u8) -> JResult<()> {
        let val = self.stack.pop().unwrap();
        let idx = usize::from(idx);
        match &val {
            StackEntry::Array(ty, _, _) => {
                writeln!(self.buf, "{}[] a{} = {{ {} }};", ty, idx, val)?;
                self.local_variables.insert(
                    idx,
                    StackEntry::Ident(format!("a{}", idx), Type::Reference(Box::new(ty.clone()))),
                );
            }
            StackEntry::String(s) => {
                writeln!(self.buf, "String s{} = \"{}\";", idx, s)?;
                self.local_variables.insert(
                    idx,
                    StackEntry::Ident(
                        format!("s{}", idx),
                        Type::ClassName("java/lang/String".to_owned()),
                    ),
                );
            }
            StackEntry::Field(ty, f) => {
                writeln!(self.buf, "{} f{} = {};", &ty, idx, f)?;
                self.local_variables
                    .insert(idx, StackEntry::Ident(format!("f{}", idx), ty.clone()));
            }
            StackEntry::Index(_, _, ty) => {
                writeln!(self.buf, "{} i{} = {};", ty, idx, val)?;
                self.local_variables
                    .insert(idx, StackEntry::Ident(format!("i{}", idx), ty.clone()));
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    fn istore(&mut self, idx: u8) -> JResult<()> {
        self.local_variables.insert(
            usize::from(idx),
            StackEntry::Ident(format!("i{}", idx), Type::Int),
        );
        writeln!(self.buf, "int i{} = {};", idx, self.stack.pop().unwrap())?;
        Ok(())
    }

    fn fstore(&mut self, idx: u8) -> JResult<()> {
        self.local_variables.insert(
            usize::from(idx),
            StackEntry::Ident(format!("f{}", idx), Type::Float),
        );
        writeln!(self.buf, "float f{} = {};", idx, self.stack.pop().unwrap())?;
        Ok(())
    }

    fn dstore(&mut self, idx: u8) -> JResult<()> {
        self.local_variables.insert(
            usize::from(idx),
            StackEntry::Ident(format!("d{}", idx), Type::Double),
        );
        writeln!(self.buf, "double d{} = {};", idx, self.stack.pop().unwrap())?;
        Ok(())
    }

    fn lstore(&mut self, idx: u8) -> JResult<()> {
        self.local_variables.insert(
            usize::from(idx),
            StackEntry::Ident(format!("l{}", idx), Type::Long),
        );
        writeln!(self.buf, "long l{} = {};", idx, self.stack.pop().unwrap())?;
        Ok(())
    }

    fn cast(&mut self, ty: Type) {
        let val = self.stack.pop().unwrap();
        self.stack.push(StackEntry::Cast(ty, Box::new(val)));
    }

    fn load(&mut self, idx: u8) {
        self.stack.push(
            self.local_variables
                .get(&usize::from(idx))
                .expect("expected local variable to exist")
                .clone(),
        );
    }
}

impl ClassFile {
    pub fn class_signature(&self) -> JResult<String> {
        let mut s = Vec::new();
        if self.access_flags.is_public {
            s.push("public");
        }
        if self.access_flags.is_final {
            s.push("final");
        }
        if self.access_flags.is_abstract {
            s.push("abstract");
        }
        if self.access_flags.is_synthetic {
            s.push("synthetic");
        }
        if self.access_flags.is_annotation {
            s.push("annotation");
        }
        if self.access_flags.is_interface {
            s.push("interface");
        }
        if self.access_flags.is_enum {
            s.push("enum");
        }
        s.push("class");
        s.push(self.class_name()?);
        if !self.access_flags.is_super {
            s.push("extends");
            s.push(self.super_class_name()?);
        }
        s.push("{\n");
        Ok(s.join(" "))
    }

    pub fn codegen<N: AsRef<str>, W: Write>(self, method_name: N, buf: &mut W) -> JResult<()> {
        let method = self.method_by_name(method_name)?;
        // buf.write_all(self.class_signature()?.as_bytes())?;
        buf.write_all(method.signature().as_bytes())?;
        let tokens = method.code().unwrap().lex().into_iter();
        let mut local_variables = HashMap::new();
        // when the method is not static, the first argument is an implicit `this`
        let arg_offset = if !method.access_flags.is_static() {
            local_variables.insert(
                0,
                StackEntry::Ident(
                    "this".to_owned(),
                    Type::ClassName(self.class_name()?.to_owned()),
                ),
            );
            1
        } else {
            0
        };
        for (idx, arg) in method.args.iter().enumerate() {
            local_variables.insert(
                idx + arg_offset,
                StackEntry::Ident(format!("arg{}", idx + arg_offset), arg.clone()),
            );
        }
        Codegen {
            class: self,
            buf,
            stack: Vec::new(),
            local_variables,
            tokens,
        }
        .codegen()
    }
}

fn main() -> JResult<()> {
    let out = std::process::Command::new("javac")
        .args(&["test.java"])
        .output()
        .expect("fd");
    if !out.stderr.is_empty() {
        dbg!(out);
        std::process::exit(1);
    }
    let reader = BufReader::new(File::open(TEST_CLASS_FILE_PATH)?);
    let file = ClassFile::from_bufreader(reader)?;
    // let mut outfile = File::create("testout.java")?;
    let mut outfile = std::io::stdout();

    // dbg!(&file.method_by_name("main"));

    dbg!(file.method_by_name("main").unwrap().code().unwrap().lex());
    file.codegen("main", &mut outfile)?;
    // file.codegen("hi", &mut outfile)?;
    Ok(())
}
