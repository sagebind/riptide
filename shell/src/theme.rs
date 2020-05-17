#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Theme {
    pub name: String,
    pub prompt: Option<Prompt>,
}

impl Default for Theme {
    fn default() -> Self {
        toml::from_str(include_str!("../../themes/default.toml")).unwrap()
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Prompt {
    pub format: Option<String>,
    pub item_separator: Option<String>,
    pub item_format: Option<String>,
}
