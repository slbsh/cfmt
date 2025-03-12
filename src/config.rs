use serde::Deserialize;
use monostate::MustBe;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
pub struct Config {
	tab_spaces: TabSpaces,
	max_width:  u16,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TabSpaces {
	Spaces(u8),
	Tabs(MustBe!("tabs")),
}

impl Default for Config {
	fn default() -> Self {
		Self {
			tab_spaces: TabSpaces::Tabs(Default::default()),
			max_width:  100,
		}
	}
}

impl Config {
	pub fn deser(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
		toml::from_str(&std::fs::read_to_string(path)?)
			.map_err(|e| crate::err!("{e}"))
	}

	pub fn indent(&self) -> String {
		match &self.tab_spaces {
			TabSpaces::Spaces(n) => " ".repeat(*n as usize),
			TabSpaces::Tabs(_)   => String::from("\t"),
		}
	}
}
