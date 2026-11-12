//! 用户自定义数据源：只保存通过安全 HTTPS origin 规范化的 Edge V1 站点。
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::RwLock};
const MAX_CUSTOM_SOURCES: usize = 20;
const FILE_NAME: &str = "custom-sources.json";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomSource { pub name: String, pub url: String }
static SOURCES: Lazy<RwLock<Vec<CustomSource>>> = Lazy::new(|| RwLock::new(Vec::new()));
static FILE: Lazy<RwLock<Option<PathBuf>>> = Lazy::new(|| RwLock::new(None));
pub fn bind(dir: &std::path::Path) {
    let path = dir.join(FILE_NAME);
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(items) = serde_json::from_str::<Vec<CustomSource>>(&raw) {
            let mut clean = Vec::new();
            for item in items {
                let Some(url) = crate::net::normalize_base(&item.url) else { continue };
                if clean.iter().any(|x: &CustomSource| x.url == url) || crate::net::trusted_auth_origin(&url) { continue; }
                let name = safe_name(&item.name, &url);
                clean.push(CustomSource { name, url });
                if clean.len() >= MAX_CUSTOM_SOURCES { break; }
            }
            if let Ok(mut current) = SOURCES.write() { *current = clean; }
        }
    }
    if let Ok(mut file) = FILE.write() { *file = Some(path); }
}
fn safe_name(raw: &str, url: &str) -> String {
    let name: String = raw.trim().chars().filter(|c| !c.is_control()).take(40).collect();
    if !name.is_empty() { return name; }
    url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_owned)).unwrap_or_else(|| "自定义线路".into())
}
fn persist(items: &[CustomSource]) -> Result<(), String> {
    let path = FILE.read().map_err(|_| "数据源配置锁失败")?.clone();
    if let Some(path) = path {
        let content = serde_json::to_vec_pretty(items).map_err(|_| "数据源配置序列化失败")?;
        std::fs::write(&path, content).map_err(|_| "数据源配置保存失败")?;
    }
    Ok(())
}
pub fn list() -> Vec<CustomSource> { SOURCES.read().map(|items| items.clone()).unwrap_or_default() }
pub fn add(name: &str, raw_url: &str) -> Result<CustomSource, String> {
    let url = crate::net::normalize_base(raw_url).ok_or_else(|| "数据源地址无效：必须是公网 HTTPS 站点根地址，不能包含路径、账号或参数".to_string())?;
    if crate::net::trusted_auth_origin(&url) { return Err("该地址已属于固定线路，无需重复添加".into()); }
    let mut guard = SOURCES.write().map_err(|_| "数据源配置锁失败")?;
    if guard.iter().any(|item| item.url == url) { return Err("该数据源已经添加".into()); }
    if guard.len() >= MAX_CUSTOM_SOURCES { return Err(format!("最多保存 {MAX_CUSTOM_SOURCES} 条自定义数据源")); }
    let item = CustomSource { name: safe_name(name, &url), url };
    let mut next = guard.clone(); next.push(item.clone()); persist(&next)?; *guard = next; Ok(item)
}
pub fn remove(raw_url: &str) -> Result<(), String> {
    let url = crate::net::normalize_base(raw_url).ok_or("数据源地址无效")?;
    if crate::net::base() == url { return Err("当前正在使用该数据源，请先切换到其他线路再删除".into()); }
    let mut guard = SOURCES.write().map_err(|_| "数据源配置锁失败")?;
    if !guard.iter().any(|item| item.url == url) { return Err("找不到该自定义数据源".into()); }
    let next: Vec<_> = guard.iter().filter(|item| item.url != url).cloned().collect();
    persist(&next)?; *guard = next; Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_source_names_are_bounded_and_safe() {
        assert_eq!(safe_name("\n demo\u{0007} source ", "https://example.com"), "demo source");
        assert!(safe_name(&"x".repeat(100), "https://example.com").len() <= 40);
    }
}
