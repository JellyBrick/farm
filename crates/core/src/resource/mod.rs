use heck::AsLowerCamelCase;

use farmfe_macro_cache_item::cache_item;

use crate::module::ModuleId;

use self::resource_pot::ResourcePotId;

// pub mod ast;
pub mod meta_data;
pub mod resource_pot;
pub mod resource_pot_map;

#[cache_item]
#[derive(Debug, Clone)]
pub enum ResourceType {
  Runtime,
  Js,
  Css,
  Html,
  SourceMap(String),
  Asset(String),
  Custom(String),
}

impl ToString for ResourceType {
  fn to_string(&self) -> String {
    match *self {
      Self::Custom(ref s) => s.to_string(),
      Self::Asset(ref s) => s.to_string(),
      _ => AsLowerCamelCase(format!("{self:?}")).to_string(),
    }
  }
}

impl serde::Serialize for ResourceType {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(self.to_string().as_str())
  }
}

impl<'de> serde::Deserialize<'de> for ResourceType {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let s = <std::string::String as serde::Deserialize>::deserialize(deserializer)?;
    Ok(s.into())
  }
}

impl From<String> for ResourceType {
  fn from(s: String) -> Self {
    match s.as_str() {
      "js" => Self::Js,
      "css" => Self::Css,
      "html" => Self::Html,
      "runtime" => Self::Runtime,
      _ => Self::Custom(s),
    }
  }
}

impl ResourceType {
  pub fn to_ext(&self) -> String {
    match self {
      ResourceType::Asset(str) => str.to_string(),
      ResourceType::Custom(str) => str.to_string(),
      ResourceType::Runtime => "js".to_string(),
      ResourceType::Js => "js".to_string(),
      ResourceType::Css => "css".to_string(),
      ResourceType::Html => "html".to_string(),
      ResourceType::SourceMap(_) => "map".to_string(),
    }
  }

  pub fn to_html_tag(&self) -> String {
    match self {
      ResourceType::Asset(str) => str.to_string(),
      ResourceType::Custom(str) => str.to_string(),
      ResourceType::Runtime => "script".to_string(),
      ResourceType::Js => "script".to_string(),
      ResourceType::Css => "link".to_string(),
      ResourceType::Html => "html".to_string(),
      ResourceType::SourceMap(_) => unreachable!(),
    }
  }
}

#[cache_item]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ResourceOrigin {
  // The resource is generated by a Resource Pot.
  ResourcePot(ResourcePotId),
  // The resource is generated by a Module. Usually by static assets like images.
  Module(ModuleId),
}

impl ResourceOrigin {
  pub fn as_resource_pot(&self) -> &ResourcePotId {
    match self {
      ResourceOrigin::ResourcePot(id) => id,
      ResourceOrigin::Module(_) => panic!("ResourceOrigin is not ResourceOrigin::ResourcePot"),
    }
  }

  pub fn as_module(&self) -> &ModuleId {
    match self {
      ResourceOrigin::ResourcePot(_) => panic!("ResourceOrigin is not ResourceOrigin::Module"),
      ResourceOrigin::Module(id) => id,
    }
  }
}

#[cache_item]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
  pub name: String,
  pub bytes: Vec<u8>,
  /// whether this resource emitted, true means this resource will not present in the final production
  pub emitted: bool,
  pub resource_type: ResourceType,
  /// the origin that this resource generated from
  pub origin: ResourceOrigin,
  // #[with(Skip)]
  // pub info: Option<ResourcePotInfo>,
}

impl Default for Resource {
  fn default() -> Self {
    Self {
      name: "unknown".to_string(),
      bytes: vec![],
      emitted: false,
      resource_type: ResourceType::Custom("unknown".to_string()),
      origin: ResourceOrigin::Module("unknown".into()),
      // info: None,
    }
  }
}
