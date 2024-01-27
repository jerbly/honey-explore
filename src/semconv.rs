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
pub enum PrimitiveType {
    String,
    Int,
    Double,
    Boolean,
    ArrayOfString,
    ArrayOfInt,
    ArrayOfDouble,
    ArrayOfBoolean,
    TemplateOfString,
    TemplateOfInt,
    TemplateOfDouble,
    TemplateOfBoolean,
    TemplateOfArrayOfString,
    TemplateOfArrayOfInt,
    TemplateOfArrayOfDouble,
    TemplateOfArrayOfBoolean,
}

impl Display for PrimitiveType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::Int => write!(f, "int"),
            PrimitiveType::Double => write!(f, "double"),
            PrimitiveType::Boolean => write!(f, "boolean"),
            PrimitiveType::ArrayOfString => write!(f, "string[]"),
            PrimitiveType::ArrayOfInt => write!(f, "int[]"),
            PrimitiveType::ArrayOfDouble => write!(f, "double[]"),
            PrimitiveType::ArrayOfBoolean => write!(f, "boolean[]"),
            PrimitiveType::TemplateOfString => write!(f, "template[string]"),
            PrimitiveType::TemplateOfInt => write!(f, "template[int]"),
            PrimitiveType::TemplateOfDouble => write!(f, "template[double]"),
            PrimitiveType::TemplateOfBoolean => write!(f, "template[boolean]"),
            PrimitiveType::TemplateOfArrayOfString => write!(f, "template[string[]]"),
            PrimitiveType::TemplateOfArrayOfInt => write!(f, "template[int[]]"),
            PrimitiveType::TemplateOfArrayOfDouble => write!(f, "template[double[]]"),
            PrimitiveType::TemplateOfArrayOfBoolean => write!(f, "template[boolean[]]"),
        }
    }
}

fn deserialize_primitive_type<'de, D>(deserializer: D) -> Result<PrimitiveType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: serde_yaml::Value = serde::Deserialize::deserialize(deserializer)?;
    match s {
        serde_yaml::Value::String(s) => match s.as_str() {
            "string" => Ok(PrimitiveType::String),
            "int" => Ok(PrimitiveType::Int),
            "double" => Ok(PrimitiveType::Double),
            "boolean" => Ok(PrimitiveType::Boolean),
            "string[]" => Ok(PrimitiveType::ArrayOfString),
            "int[]" => Ok(PrimitiveType::ArrayOfInt),
            "double[]" => Ok(PrimitiveType::ArrayOfDouble),
            "boolean[]" => Ok(PrimitiveType::ArrayOfBoolean),
            "template[string]" => Ok(PrimitiveType::TemplateOfString),
            "template[int]" => Ok(PrimitiveType::TemplateOfInt),
            "template[double]" => Ok(PrimitiveType::TemplateOfDouble),
            "template[boolean]" => Ok(PrimitiveType::TemplateOfBoolean),
            "template[string[]]" => Ok(PrimitiveType::TemplateOfArrayOfString),
            "template[int[]]" => Ok(PrimitiveType::TemplateOfArrayOfInt),
            "template[double[]]" => Ok(PrimitiveType::TemplateOfArrayOfDouble),
            "template[boolean[]]" => Ok(PrimitiveType::TemplateOfArrayOfBoolean),
            _ => Err(serde::de::Error::custom("Failed to parse type")),
        },
        _ => Err(serde::de::Error::custom("Failed to parse type")),
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Type {
    #[serde(deserialize_with = "deserialize_primitive_type")]
    Simple(PrimitiveType),
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
    pub defined_in: Option<String>,
}

impl Attribute {
    pub fn is_template_type(&self) -> bool {
        matches!(
            &self.r#type,
            Some(Type::Simple(PrimitiveType::TemplateOfString))
                | Some(Type::Simple(PrimitiveType::TemplateOfInt))
                | Some(Type::Simple(PrimitiveType::TemplateOfDouble))
                | Some(Type::Simple(PrimitiveType::TemplateOfBoolean))
                | Some(Type::Simple(PrimitiveType::TemplateOfArrayOfString))
                | Some(Type::Simple(PrimitiveType::TemplateOfArrayOfInt))
                | Some(Type::Simple(PrimitiveType::TemplateOfArrayOfDouble))
                | Some(Type::Simple(PrimitiveType::TemplateOfArrayOfBoolean))
        )
    }
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
    pub fn new(root_dirs: &[(String, String)]) -> anyhow::Result<Self> {
        let mut sc = SemanticConventions {
            attribute_map: HashMap::new(),
        };
        for (nick_name, root_dir) in root_dirs {
            let yml = format!("{root_dir}/**/*.yml");
            let yaml = format!("{root_dir}/**/*.yaml");
            for entry in glob(yml.as_str())?.chain(glob(yaml.as_str())?) {
                let entry = entry?;
                // print the trailing part of the path after the root_dir
                let defined_in = format!(
                    "{}::{}",
                    nick_name,
                    &entry.strip_prefix(root_dir)?.to_str().unwrap_or("")
                );
                sc.read_file(entry, defined_in)?;
            }
        }
        Ok(sc)
    }

    pub fn read_file(&mut self, path: PathBuf, defined_in: String) -> anyhow::Result<()> {
        println!("reading file: {:?}", path);
        let groups: Groups = serde_yaml::from_reader(&File::open(path)?)?;
        for group in groups.groups {
            if let Some(attributes) = group.attributes {
                for mut attribute in attributes {
                    let is_some = attribute.id.is_some();
                    if is_some {
                        attribute.defined_in = Some(defined_in.clone());
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
