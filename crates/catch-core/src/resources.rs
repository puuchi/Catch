use crate::{tracking::text_length, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    File,
    Folder,
    Web,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub target: String,
}
fn path_target(value: &str) -> bool {
    if value.starts_with(r"\\.\") || value.to_ascii_uppercase().starts_with(r"\\?\GLOBALROOT") {
        return false;
    }
    let value = value.strip_prefix(r"\\?\").unwrap_or(value);
    let bytes = value.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    let unc = value
        .strip_prefix(r"\\")
        .or_else(|| value.strip_prefix(r"UNC\"))
        .is_some_and(|unc| {
            let parts: Vec<_> = unc.split('\\').filter(|part| !part.is_empty()).collect();
            parts.len() >= 2 && parts[0] != "?" && parts[0] != "."
        });
    drive || unc || value.starts_with('/')
}
impl Resource {
    pub fn new(id: &str, kind: Kind, target: &str, title: Option<&str>) -> Result<Self> {
        let target = target.trim();
        let title = match title.map(str::trim).filter(|title| !title.is_empty()) {
            Some(title) => title.to_string(),
            None => {
                if kind == Kind::Web {
                    Url::parse(target)
                        .ok()
                        .and_then(|url| url.host_str().map(str::to_owned))
                        .unwrap_or_default()
                } else {
                    target
                        .trim_end_matches(['\\', '/'])
                        .rsplit(['\\', '/'])
                        .next()
                        .unwrap_or(target)
                        .into()
                }
            }
        };
        let resource = Self {
            id: id.into(),
            kind,
            title,
            target: target.into(),
        };
        resource.validate()?;
        Ok(resource)
    }
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() || text_length(&self.id) > 100 {
            return Err("资料标识无效".into());
        }
        if self.title.trim().is_empty() || text_length(&self.title) > 200 {
            return Err("资料名称须为 1–200 个字符".into());
        }
        if self.target.is_empty()
            || text_length(&self.target) > 32767
            || self
                .target
                .chars()
                .any(|ch| ch == '\0' || ch == '\r' || ch == '\n')
        {
            return Err("资料地址无效".into());
        }
        if self.kind == Kind::Web {
            let url = Url::parse(&self.target).map_err(|_| "网页地址无效")?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err("请使用 http 或 https 网页地址".into());
            }
        } else if !path_target(&self.target) {
            return Err("请选择文件或文件夹的完整路径".into());
        }
        Ok(())
    }
    pub fn relocated(&self, target: &str) -> Result<Self> {
        let mut next = self.clone();
        next.target = target.trim().into();
        next.validate()?;
        Ok(next)
    }
    fn identity(&self) -> String {
        if self.kind == Kind::Web {
            Url::parse(&self.target)
                .map(|url| url.to_string())
                .unwrap_or_else(|_| self.target.clone())
        } else {
            self.target
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_lowercase()
        }
    }
}
pub fn validate_list(resources: &[Resource]) -> Result<()> {
    if resources.len() > 32 {
        return Err("每项最多关联 32 份资料".into());
    }
    let mut ids = HashSet::new();
    let mut targets = HashSet::new();
    for resource in resources {
        resource.validate()?;
        if !ids.insert(&resource.id) {
            return Err("资料标识重复".into());
        }
        if !targets.insert(resource.identity()) {
            return Err("这份资料已经关联".into());
        }
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Owner {
    Todo { id: String },
    Matter { id: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    Attach {
        owner: Owner,
        kind: Kind,
        target: String,
        #[serde(default)]
        title: Option<String>,
    },
    Remove {
        owner: Owner,
        resource_id: String,
    },
    Relocate {
        owner: Owner,
        resource_id: String,
        target: String,
        #[serde(default)]
        title: Option<String>,
    },
}
impl Command {
    pub fn owner(&self) -> &Owner {
        match self {
            Self::Attach { owner, .. }
            | Self::Remove { owner, .. }
            | Self::Relocate { owner, .. } => owner,
        }
    }
    pub fn apply(&self, current: &[Resource], new_id: &str) -> Result<Vec<Resource>> {
        let mut next = current.to_vec();
        match self {
            Self::Attach {
                kind,
                target,
                title,
                ..
            } => next.push(Resource::new(new_id, *kind, target, title.as_deref())?),
            Self::Remove { resource_id, .. } => {
                let index = next
                    .iter()
                    .position(|resource| resource.id == *resource_id)
                    .ok_or("资料关联已不存在")?;
                next.remove(index);
            }
            Self::Relocate {
                resource_id,
                target,
                title,
                ..
            } => {
                let index = next
                    .iter()
                    .position(|resource| resource.id == *resource_id)
                    .ok_or("资料关联已不存在")?;
                next[index] = next[index].relocated(target)?;
                if let Some(title) = title {
                    next[index].title = title.trim().into();
                }
            }
        }
        validate_list(&next)?;
        Ok(next)
    }
}
