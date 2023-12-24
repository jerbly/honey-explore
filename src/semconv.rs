use glob::glob;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    fs::File,
    path::PathBuf,
};

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MemberValue {
    StringType(String),
    IntegerType(i64),
}

impl Display for MemberValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberValue::StringType(s) => write!(f, "{}", s),
            MemberValue::IntegerType(i) => write!(f, "{}", i),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Member {
    pub value: MemberValue,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ComplexType {
    #[serde(default)]
    pub allow_custom_values: bool,
    pub members: Vec<Member>,
}

impl Display for ComplexType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.allow_custom_values {
            write!(f, "open enum: ")?;
        } else {
            write!(f, "enum: ")?;
        }

        let values: Vec<String> = self
            .members
            .iter()
            .map(|member| format!("{}", member.value))
            .collect();
        write!(f, "{}", values.join(", "))
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Type {
    Simple(String),
    Complex(ComplexType),
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Simple(s) => write!(f, "{}", s),
            Type::Complex(c) => write!(f, "{}", c),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ValueType {
    String(String),
    Bool(bool),
    Integer(i64),
    Float(f64),
    ArrayString(Vec<String>),
    ArrayBool(Vec<bool>),
    ArrayInteger(Vec<i64>),
    ArrayFloat(Vec<f64>),
}

impl Display for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::String(s) => write!(f, "{}", s),
            ValueType::Bool(b) => write!(f, "{}", b),
            ValueType::Integer(i) => write!(f, "{}", i),
            ValueType::Float(fl) => write!(f, "{}", fl),
            ValueType::ArrayString(s) => write!(f, "{:?}", s),
            ValueType::ArrayBool(b) => write!(f, "{:?}", b),
            ValueType::ArrayInteger(i) => write!(f, "{:?}", i),
            ValueType::ArrayFloat(fl) => write!(f, "{:?}", fl),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Examples {
    ArrayType(Vec<ValueType>),
    SimpleType(ValueType),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Attribute {
    pub id: Option<String>,
    pub r#type: Option<Type>,
    pub brief: Option<String>,
    pub note: Option<String>,
    pub examples: Option<Examples>,
    pub deprecated: Option<String>,
    pub used_by: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Group {
    prefix: Option<String>,
    attributes: Option<Vec<Attribute>>,
}

#[derive(Debug, Deserialize)]
struct Groups {
    groups: Vec<Group>,
}

#[derive(Debug)]
pub struct SemanticConventions {
    pub attribute_map: HashMap<String, Attribute>,
}

impl SemanticConventions {
    pub fn new(root_dirs: &[String]) -> anyhow::Result<Self> {
        let mut sc = SemanticConventions {
            attribute_map: HashMap::new(),
        };
        for root_dir in root_dirs {
            let yml = format!("{root_dir}/**/*.yml");
            let yaml = format!("{root_dir}/**/*.yaml");
            for entry in glob(yml.as_str())?.chain(glob(yaml.as_str())?) {
                sc.read_file(entry?)?;
            }
        }
        Ok(sc)
    }

    pub fn read_file(&mut self, path: PathBuf) -> anyhow::Result<()> {
        println!("reading file: {:?}", path);
        let groups: Groups = serde_yaml::from_reader(&File::open(path)?)?;
        for group in groups.groups {
            if let Some(attributes) = group.attributes {
                for attribute in attributes {
                    let is_some = attribute.id.is_some();
                    if is_some {
                        self.attribute_map.insert(
                            match &group.prefix {
                                Some(prefix) => {
                                    format!("{}.{}", prefix, attribute.id.clone().unwrap())
                                }
                                None => attribute.id.clone().unwrap(),
                            },
                            attribute,
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
