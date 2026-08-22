//! Template fixtures (§7): versioned provider payloads with `{{placeholder}}`
//! interpolation. Bundled sets ship embedded; user dirs override by ref.

use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TemplateFile {
    pub v: u32,
    pub provider: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub signing_hint: Option<String>,
    pub request: TemplateRequest,
    #[serde(default)]
    pub defaults: Defaults,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TemplateRequest {
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: serde_json::Value,
}

fn default_method() -> String {
    "POST".into()
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Defaults {
    #[serde(default)]
    pub vars: BTreeMap<String, serde_json::Value>,
}

impl TemplateFile {
    pub fn r#ref(&self) -> String {
        format!("{}/{}@{}", self.provider, self.name, self.v)
    }
}

/// Interpolate `{{name}}` placeholders. Resolution order per §7: request vars →
/// template defaults → error listing every missing name. Builtins: uuid, now_unix, now_iso.
pub fn interpolate(
    value: &serde_json::Value,
    vars: &BTreeMap<String, serde_json::Value>,
    template_vars: &BTreeMap<String, serde_json::Value>,
) -> Result<serde_json::Value, Vec<String>> {
    let now = crate::envelope::truncate_ms(crate::envelope::now_utc());
    let mut missing = Vec::new();
    let resolved = interp_value(value, vars, template_vars, &mut missing, now);
    if missing.is_empty() {
        Ok(resolved)
    } else {
        missing.sort();
        missing.dedup();
        Err(missing)
    }
}

fn lookup<'a>(
    name: &str,
    vars: &'a BTreeMap<String, serde_json::Value>,
    template_vars: &'a BTreeMap<String, serde_json::Value>,
) -> Option<&'a serde_json::Value> {
    vars.get(name).or_else(|| template_vars.get(name))
}

fn builtin(name: &str, now: time::OffsetDateTime) -> Option<String> {
    match name {
        "uuid" | "ulid" => Some(ulid::Ulid::new().to_string()),
        "now_unix" => Some(now.unix_timestamp().to_string()),
        "now_iso" => Some(crate::envelope::format_rfc3339_ms(now)),
        _ => None,
    }
}

fn render_text(
    text: &str,
    vars: &BTreeMap<String, serde_json::Value>,
    tpl: &BTreeMap<String, serde_json::Value>,
    missing: &mut Vec<String>,
    now: time::OffsetDateTime,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                let name = after[..end].trim();
                let rendered = if let Some(v) = lookup(name, vars, tpl) {
                    match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    }
                } else if let Some(b) = builtin(name, now) {
                    b
                } else {
                    missing.push(name.to_string());
                    format!("{{{{{name}}}}}")
                };
                out.push_str(&rendered);
                rest = &after[end + 2..];
            }
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn interp_value(
    v: &serde_json::Value,
    vars: &BTreeMap<String, serde_json::Value>,
    tpl: &BTreeMap<String, serde_json::Value>,
    missing: &mut Vec<String>,
    now: time::OffsetDateTime,
) -> serde_json::Value {
    match v {
        serde_json::Value::String(s) => {
            // Whole-string placeholder keeps non-string type when it stands alone.
            let trimmed = s.trim();
            if trimmed.len() > 4 && trimmed.starts_with("{{") && trimmed.ends_with("}}") {
                let name = &trimmed[2..trimmed.len() - 2];
                if !name.contains('{') {
                    if let Some(val) = lookup(name.trim(), vars, tpl) {
                        return val.clone();
                    }
                    if builtin(name.trim(), now).is_none() {
                        missing.push(name.trim().to_string());
                    }
                    return serde_json::Value::String(s.clone());
                }
            }
            serde_json::Value::String(render_text(s, vars, tpl, missing, now))
        }
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, val)| (k.clone(), interp_value(val, vars, tpl, missing, now)))
                .collect(),
        ),
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|i| interp_value(i, vars, tpl, missing, now))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Registry of bundled + user-supplied templates, keyed by `provider/name`.
pub struct Registry {
    templates: Vec<Arc<TemplateFile>>,
}

impl Registry {
    /// `bundled`: embedded starter sets; `dir`: optional override directory laid out
    /// as `{provider}/{name}.json` (§7).
    pub fn load(bundled: &[&str], dir: Option<&std::path::Path>) -> anyhow::Result<Self> {
        let mut templates = Vec::new();
        for raw in bundled {
            let t: TemplateFile = serde_json::from_str(raw)?;
            templates.push(Arc::new(t));
        }
        if let Some(dir) = dir {
            let mut stack = vec![dir.to_path_buf()];
            while let Some(d) = stack.pop() {
                for entry in std::fs::read_dir(&d)?.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().is_some_and(|e| e == "json") {
                        match std::fs::read_to_string(&p)
                            .map_err(anyhow::Error::from)
                            .and_then(|s| {
                                serde_json::from_str::<TemplateFile>(&s)
                                    .map_err(anyhow::Error::from)
                            }) {
                            Ok(t) => templates.push(Arc::new(t)),
                            Err(e) => {
                                tracing::warn!(path = %p.display(), error = %e, "skipping bad template")
                            }
                        }
                    }
                }
            }
        }
        Ok(Self { templates })
    }

    /// Resolve `provider/name` or `provider/name@version`.
    pub fn get(&self, r#ref: &str) -> Option<Arc<TemplateFile>> {
        let (base, version) = match r#ref.rsplit_once('@') {
            Some((b, v)) => (b, v.parse::<u32>().ok()),
            None => (r#ref, None),
        };
        self.templates
            .iter()
            .filter(|t| format!("{}/{}", t.provider, t.name) == base)
            .filter(|t| version.is_none_or(|v| t.v == v))
            .max_by_key(|t| t.v)
            .cloned()
    }

    pub fn list(&self) -> Vec<Arc<TemplateFile>> {
        self.templates.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
            .collect()
    }

    #[test]
    fn interpolation_resolves_vars_then_defaults_then_errors() {
        let tpl_vars = vars(&[("customer", "cus_default")]);
        let body = serde_json::json!({
            "customer": "{{customer}}",
            "id": "evt_{{uuid}}",
            "created": "{{now_unix}}",
            "note": "hello {{who}} and {{customer}}",
            "amount": 42
        });

        // falls back to defaults for customer; who is missing entirely
        let err = interpolate(&body, &vars(&[]), &tpl_vars).unwrap_err();
        assert_eq!(err, vec!["who".to_string()]);

        let ok = interpolate(&body, &vars(&[("who", "world")]), &tpl_vars).unwrap();
        assert_eq!(ok["customer"], "cus_default"); // default applied
        assert_eq!(ok["note"], "hello world and cus_default");
        assert_eq!(ok["amount"], 42); // non-string untouched
        let id = ok["id"].as_str().unwrap();
        assert!(id.starts_with("evt_") && id.len() > 10); // {{uuid}} builtin fired

        // explicit var beats default
        let ok2 = interpolate(
            &body,
            &vars(&[("customer", "cus_real"), ("who", "x")]),
            &tpl_vars,
        )
        .unwrap();
        assert_eq!(ok2["customer"], "cus_real");
    }

    #[test]
    fn whole_string_placeholder_keeps_type() {
        let tpl_vars = Default::default();
        let v = serde_json::json!({"count": "{{count}}"});
        let ok = interpolate(&v, &vars(&[("count", "7")]), &tpl_vars).unwrap();
        // string-typed vars stay strings; numeric JSON vars keep their type
        assert_eq!(ok["count"], "7");
        let v2 = serde_json::json!({"count": "{{count}}"});
        let numeric = BTreeMap::from([("count".to_string(), serde_json::json!(7))]);
        let ok2 = interpolate(&v2, &numeric, &tpl_vars).unwrap();
        assert_eq!(ok2["count"], 7);
    }

    #[test]
    fn registry_resolves_refs_with_versions() {
        let a: TemplateFile = serde_json::from_str(
            r#"{"v":1,"provider":"stripe","name":"invoice.paid","request":{"method":"POST","path":"/p"}}"#,
        )
        .unwrap();
        let reg = Registry {
            templates: vec![Arc::new(a)],
        };
        assert!(reg.get("stripe/invoice.paid").is_some());
        assert!(reg.get("stripe/invoice.paid@1").is_some());
        assert!(reg.get("stripe/invoice.paid@9").is_none());
        assert!(reg.get("stripe/nope").is_none());
    }
}
