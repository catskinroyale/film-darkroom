//! Built-in and user styles. User styles persist as JSON in the platform config
//! directory (`~/Library/Application Support/film-darkroom/styles.json` on macOS).
//!
//! Until that file exists, the user styles of BW Darkroom (`bw-darkroom/styles.json`) and
//! Colour Darkroom (`colour-darkroom/styles.json`) are read in as a starting point. Those
//! files are only ever read.

use std::path::{Path, PathBuf};

use fd_profiles::{Process, Style, builtin_styles};

pub struct StyleEntry {
    pub style: Style,
    pub builtin: bool,
}

pub struct StyleStore {
    pub entries: Vec<StyleEntry>,
    path: Option<PathBuf>,
}

/// Read a styles file: a list of styles, tagged with their process or (from the
/// single-process apps) not. Styles that can't be read are skipped.
fn read_styles(path: &Path) -> Option<Vec<Style>> {
    let text = std::fs::read_to_string(path).ok()?;
    let values: Vec<serde_json::Value> = serde_json::from_str(&text).ok()?;
    Some(
        values
            .into_iter()
            .filter_map(|v| Style::from_value(v).ok())
            .collect(),
    )
}

impl StyleStore {
    pub fn load() -> Self {
        let config = dirs::config_dir();
        let path = config
            .as_ref()
            .map(|d| d.join("film-darkroom").join("styles.json"));
        let legacy: Vec<PathBuf> = config
            .iter()
            .flat_map(|d| {
                ["bw-darkroom", "colour-darkroom"].map(|app| d.join(app).join("styles.json"))
            })
            .collect();
        Self::load_from(path, &legacy)
    }

    /// The built-ins plus the user styles in `path`, or, while `path` doesn't exist yet,
    /// those in the `legacy` files (renamed where names clash).
    pub fn load_from(path: Option<PathBuf>, legacy: &[PathBuf]) -> Self {
        let mut store = Self {
            entries: builtin_styles()
                .into_iter()
                .map(|style| StyleEntry {
                    style,
                    builtin: true,
                })
                .collect(),
            path,
        };
        match &store.path {
            Some(p) if p.exists() => {
                let user = read_styles(p).unwrap_or_default();
                store
                    .entries
                    .extend(user.into_iter().map(|style| StyleEntry {
                        style,
                        builtin: false,
                    }));
            }
            _ => {
                for file in legacy {
                    store.import(read_styles(file).unwrap_or_default());
                }
            }
        }
        store
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let user: Vec<&Style> = self
            .entries
            .iter()
            .filter(|e| !e.builtin)
            .map(|e| &e.style)
            .collect();
        std::fs::write(path, serde_json::to_string_pretty(&user)?)
    }

    /// The styles of one process, with their indices.
    pub fn styles_of(&self, process: Process) -> Vec<(usize, Style)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.style.process() == process)
            .map(|(i, e)| (i, e.style.clone()))
            .collect()
    }

    /// A name not used by any existing style.
    pub fn unique_name(&self, base: &str) -> String {
        let taken = |n: &str| self.entries.iter().any(|e| e.style.name() == n);
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|i| format!("{base} ({i})"))
            .find(|n| !taken(n))
            .expect("unbounded search")
    }

    /// Copy style `index` as a new user style named "<name> copy", placed after it
    /// (after the built-ins when the original is built in). Returns the new index.
    pub fn duplicate(&mut self, index: usize) -> usize {
        let mut style = self.entries[index].style.clone();
        style.set_name(self.unique_name(&format!("{} copy", style.name())));
        let first_user = self
            .entries
            .iter()
            .position(|e| !e.builtin)
            .unwrap_or(self.entries.len());
        let at = (index + 1).max(first_user);
        self.entries.insert(
            at,
            StyleEntry {
                style,
                builtin: false,
            },
        );
        at
    }

    /// Rename a user style. Names must be non-empty and unique.
    pub fn rename(&mut self, index: usize, name: &str) -> Result<(), String> {
        let name = name.trim();
        if self.entries[index].builtin {
            return Err("Built-in styles can't be renamed: duplicate it first.".into());
        }
        if name.is_empty() {
            return Err("Enter a name.".into());
        }
        if self
            .entries
            .iter()
            .enumerate()
            .any(|(i, e)| i != index && e.style.name() == name)
        {
            return Err("Another style already has that name.".into());
        }
        self.entries[index].style.set_name(name);
        Ok(())
    }

    /// Delete a user style.
    pub fn delete(&mut self, index: usize) -> Result<StyleEntry, String> {
        if self.entries[index].builtin {
            return Err("Built-in styles can't be deleted.".into());
        }
        Ok(self.entries.remove(index))
    }

    /// Whether user style `index` can move one place by `delta` (−1 up, +1 down)
    /// without crossing into the built-ins or off the end.
    pub fn can_move(&self, index: usize, delta: isize) -> bool {
        let target = index as isize + delta;
        !self.entries[index].builtin
            && target >= 0
            && (target as usize) < self.entries.len()
            && !self.entries[target as usize].builtin
    }

    /// Move a user style one place up or down. Returns its new index.
    pub fn move_by(&mut self, index: usize, delta: isize) -> Option<usize> {
        if !self.can_move(index, delta) {
            return None;
        }
        let target = (index as isize + delta) as usize;
        self.entries.swap(index, target);
        Some(target)
    }

    /// Add styles read from a file as user styles, renaming any that clash.
    /// Returns the index of the first one added.
    pub fn import(&mut self, styles: Vec<Style>) -> Option<usize> {
        let first = self.entries.len();
        for mut style in styles {
            style.set_name(self.unique_name(style.name()));
            self.entries.push(StyleEntry {
                style,
                builtin: false,
            });
        }
        (self.entries.len() > first).then_some(first)
    }
}

/// Parse a style file: one style or a list of styles (JSON), from this app or from BW
/// Darkroom or Colour Darkroom.
pub fn parse_style_file(text: &str) -> Result<Vec<Style>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("Not a style file: {e}"))?;
    let values = match value {
        serde_json::Value::Array(values) => values,
        one => vec![one],
    };
    values
        .into_iter()
        .map(|v| Style::from_value(v).map_err(|e| format!("Not a style file: {e}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> StyleStore {
        let mut entries: Vec<StyleEntry> = builtin_styles()
            .into_iter()
            .take(2)
            .map(|style| StyleEntry {
                style,
                builtin: true,
            })
            .collect();
        let mut mine = entries[0].style.clone();
        mine.set_name("Mine");
        entries.push(StyleEntry {
            style: mine,
            builtin: false,
        });
        StyleStore {
            entries,
            path: None,
        }
    }

    #[test]
    fn duplicate_goes_after_builtins_with_unique_name() {
        let mut s = store();
        let name = s.entries[0].style.name().to_string();
        let i = s.duplicate(0);
        assert_eq!(i, 2, "after the built-ins");
        assert_eq!(s.entries[i].style.name(), format!("{name} copy"));
        assert!(!s.entries[i].builtin);
        let j = s.duplicate(0);
        assert_eq!(s.entries[j].style.name(), format!("{name} copy (2)"));
        let k = s.duplicate(3);
        assert_eq!(k, 4, "user styles duplicate in place");
    }

    #[test]
    fn rename_and_delete_rules() {
        let mut s = store();
        assert!(s.rename(0, "x").is_err(), "built-in");
        assert!(s.rename(2, "  ").is_err(), "empty");
        let taken = s.entries[1].style.name().to_string();
        assert!(s.rename(2, &taken).is_err(), "duplicate");
        s.rename(2, "  Darker ").unwrap();
        assert_eq!(s.entries[2].style.name(), "Darker");
        assert!(s.delete(0).is_err());
        assert_eq!(s.delete(2).unwrap().style.name(), "Darker");
        assert_eq!(s.entries.len(), 2);
    }

    #[test]
    fn user_styles_reorder_without_crossing_builtins() {
        let mut s = store();
        s.duplicate(2);
        assert!(!s.can_move(2, -1), "cannot move above the built-ins");
        assert_eq!(s.move_by(2, 1), Some(3));
        assert_eq!(s.entries[3].style.name(), "Mine");
        assert_eq!(s.move_by(3, 1), None, "already last");
        assert_eq!(s.move_by(0, 1), None, "built-ins don't move");
    }

    #[test]
    fn first_launch_reads_both_apps_styles_and_never_writes_them() {
        let dir = std::env::temp_dir().join(format!("fd-styles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Untagged, as BW Darkroom and Colour Darkroom write them.
        let mut bw = fd_profiles::mono::builtin_styles().remove(0);
        bw.name = "My Tri-X".into();
        let mut colour = fd_profiles::colour::builtin_styles().remove(0);
        colour.name = "My Portra".into();
        let bw_file = dir.join("bw.json");
        let colour_file = dir.join("colour.json");
        let bw_text = serde_json::to_string(&vec![&bw]).unwrap();
        std::fs::write(&bw_file, &bw_text).unwrap();
        std::fs::write(&colour_file, serde_json::to_string(&vec![&colour]).unwrap()).unwrap();
        let ours = dir.join("film-darkroom").join("styles.json");
        let legacy = [bw_file.clone(), colour_file.clone()];

        let s = StyleStore::load_from(Some(ours.clone()), &legacy);
        let mine: Vec<(&str, Process)> = s
            .entries
            .iter()
            .filter(|e| !e.builtin)
            .map(|e| (e.style.name(), e.style.process()))
            .collect();
        assert_eq!(
            mine,
            [("My Tri-X", Process::Mono), ("My Portra", Process::Colour)]
        );
        s.save().unwrap();
        assert!(
            std::fs::read_to_string(&ours)
                .unwrap()
                .contains("\"process\"")
        );
        assert_eq!(std::fs::read_to_string(&bw_file).unwrap(), bw_text);

        // Once our file exists, the old ones are no longer read.
        std::fs::write(&bw_file, "[]").unwrap();
        let again = StyleStore::load_from(Some(ours), &legacy);
        assert_eq!(again.entries.iter().filter(|e| !e.builtin).count(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn import_accepts_one_or_many_and_renames_clashes() {
        let mut s = store();
        let one = serde_json::to_string(&s.entries[2].style).unwrap();
        let first = s.import(parse_style_file(&one).unwrap()).unwrap();
        assert_eq!(s.entries[first].style.name(), "Mine (2)");
        let many = serde_json::to_string(&vec![
            s.entries[0].style.clone(),
            s.entries[1].style.clone(),
        ])
        .unwrap();
        assert_eq!(parse_style_file(&many).unwrap().len(), 2);
        assert!(parse_style_file("{\"nope\": 1}").is_err());
    }
}
