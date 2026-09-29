//! [`KeyChord`]: a physical key combination, with parsing and formatting for the
//! `"Ctrl+Shift+O"` strings a config file stores, and matching against raw wx key codes.
//!
//! This crate is deliberately dependency-free (`serde` is an optional feature) so that a
//! GUI-free core library can own an app's keymap and defaults without linking a toolkit,
//! while the same type reaches the shortcut-editing UI unchanged.

#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]

/// The key codes wx reports, from its `WXK_*` constants.
const WXK_F1: i32 = 340;
const WXK_F24: i32 = 363;
const WXK_NUMPAD0: i32 = 324;
const WXK_NUMPAD9: i32 = 333;

/// Each named key with the wx key codes that produce it: its own, then its numpad equivalent,
/// which is what the numpad sends with Num Lock off, and for punctuation the Windows virtual key
/// code some keyboard layouts report.
const NAMED_KEYS: &[(&str, &[i32])] = &[
	("Enter", &[13, 370]),
	("Tab", &[9, 369]),
	("Space", &[32, 368]),
	("Backspace", &[8]),
	("Delete", &[127, 385]),
	("Escape", &[27]),
	("Home", &[313, 375]),
	("End", &[312, 382]),
	("PageUp", &[366, 380]),
	("PageDown", &[367, 381]),
	("Left", &[314, 376]),
	("Up", &[315, 377]),
	("Right", &[316, 378]),
	("Down", &[317, 379]),
	(",", &[44, 188]),
	(".", &[46, 190, 391]),
	("/", &[47, 191, 392]),
	("[", &[91, 219]),
	("]", &[93, 221]),
	("\\", &[92, 220]),
	("-", &[45, 189, 390]),
	("=", &[61, 187, 386]),
	(";", &[59, 186]),
	("'", &[39, 222]),
	("`", &[96, 192]),
];

/// A modifier combination plus one key.
///
/// `ctrl` and `raw_ctrl` are separate because macOS has two keys where Windows and Linux have
/// one. `ctrl` is the *command* modifier, which wx maps to Cmd on macOS and to Ctrl elsewhere,
/// and is what an ordinary menu accelerator wants. `raw_ctrl` is the physical Control key,
/// which on macOS is a different key and on other platforms is the same one. A binding only
/// needs `raw_ctrl` when it has to stay physical Control on a Mac, so [`new`](KeyChord::new)
/// produces a `ctrl` chord and [`new_raw_ctrl`](KeyChord::new_raw_ctrl) is the exception you
/// reach for deliberately.
///
/// `win` is the Windows logo key. It only makes sense for a system-wide hotkey: wx key events
/// don't report it, and Windows keeps many Win combinations for itself before a window sees them,
/// so [`matches`](KeyChord::matches) never fires for a `win` chord.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KeyChord {
	pub ctrl: bool,
	#[cfg_attr(feature = "serde", serde(default))]
	pub raw_ctrl: bool,
	pub alt: bool,
	pub shift: bool,
	#[cfg_attr(feature = "serde", serde(default))]
	pub win: bool,
	pub key: String,
}

impl KeyChord {
	/// A chord on the command modifier: Cmd on macOS, Ctrl elsewhere.
	#[must_use]
	pub fn new(ctrl: bool, alt: bool, shift: bool, key: impl Into<String>) -> Self {
		let key_str = key.into();
		let normalized = Self::normalize_key_name(&key_str);
		Self { ctrl, raw_ctrl: false, alt, shift, win: false, key: normalized }
	}

	/// A chord on the physical Control key, which stays Control on macOS rather than becoming Cmd.
	#[must_use]
	pub fn new_raw_ctrl(raw_ctrl: bool, alt: bool, shift: bool, key: impl Into<String>) -> Self {
		let key_str = key.into();
		let normalized = Self::normalize_key_name(&key_str);
		Self { ctrl: false, raw_ctrl, alt, shift, win: false, key: normalized }
	}

	/// This chord with the Windows logo key added or removed.
	#[must_use]
	pub const fn with_win(mut self, win: bool) -> Self {
		self.win = win;
		self
	}

	/// Canonical spelling of a key name, so `"esc"`, `"Escape"`, and `"ESC"` all compare equal,
	/// and a config file edited by hand still matches.
	#[must_use]
	pub fn normalize_key_name(key: &str) -> String {
		let trimmed = key.trim();
		if trimmed.eq_ignore_ascii_case("return") || trimmed.eq_ignore_ascii_case("enter") {
			"Enter".to_string()
		} else if trimmed.eq_ignore_ascii_case("space") {
			"Space".to_string()
		} else if trimmed.eq_ignore_ascii_case("tab") {
			"Tab".to_string()
		} else if trimmed.eq_ignore_ascii_case("backspace") || trimmed.eq_ignore_ascii_case("back") {
			"Backspace".to_string()
		} else if trimmed.eq_ignore_ascii_case("delete") || trimmed.eq_ignore_ascii_case("del") {
			"Delete".to_string()
		} else if trimmed.eq_ignore_ascii_case("escape") || trimmed.eq_ignore_ascii_case("esc") {
			"Escape".to_string()
		} else if trimmed.eq_ignore_ascii_case("home") {
			"Home".to_string()
		} else if trimmed.eq_ignore_ascii_case("end") {
			"End".to_string()
		} else if trimmed.eq_ignore_ascii_case("pageup")
			|| trimmed.eq_ignore_ascii_case("page up")
			|| trimmed.eq_ignore_ascii_case("pgup")
		{
			"PageUp".to_string()
		} else if trimmed.eq_ignore_ascii_case("pagedown")
			|| trimmed.eq_ignore_ascii_case("page down")
			|| trimmed.eq_ignore_ascii_case("pgdn")
		{
			"PageDown".to_string()
		} else if trimmed.eq_ignore_ascii_case("left") || trimmed.eq_ignore_ascii_case("left arrow") {
			"Left".to_string()
		} else if trimmed.eq_ignore_ascii_case("right") || trimmed.eq_ignore_ascii_case("right arrow") {
			"Right".to_string()
		} else if trimmed.eq_ignore_ascii_case("up") || trimmed.eq_ignore_ascii_case("up arrow") {
			"Up".to_string()
		} else if trimmed.eq_ignore_ascii_case("down") || trimmed.eq_ignore_ascii_case("down arrow") {
			"Down".to_string()
		} else if trimmed.len() >= 2
			&& trimmed.starts_with(['F', 'f'])
			&& trimmed[1..].chars().all(|c| c.is_ascii_digit())
		{
			format!("F{}", &trimmed[1..])
		} else if trimmed.len() == 1 {
			let ch = trimmed.chars().next().unwrap_or_default();
			if ch.is_ascii_alphabetic() { ch.to_ascii_uppercase().to_string() } else { trimmed.to_string() }
		} else {
			trimmed.to_string()
		}
	}

	/// The `"Ctrl+Shift+O"` form, which is both what the user reads and what a config file stores.
	#[must_use]
	pub fn to_shortcut_string(&self) -> String {
		let mut parts = Vec::new();
		if self.raw_ctrl {
			parts.push("RawCtrl");
		}
		if self.ctrl {
			parts.push("Ctrl");
		}
		if self.alt {
			parts.push("Alt");
		}
		if self.shift {
			parts.push("Shift");
		}
		if self.win {
			parts.push("Win");
		}
		parts.push(&self.key);
		parts.join("+")
	}

	/// The chord as the user should read it, which differs from
	/// [`to_shortcut_string`](KeyChord::to_shortcut_string) on macOS: the command modifier is Cmd,
	/// Alt is Option, and physical Control is Control, in the order Mac menus list them. Elsewhere
	/// both kinds of Ctrl read as Ctrl.
	#[must_use]
	pub fn to_display_string(&self) -> String {
		let modifiers = if cfg!(target_os = "macos") {
			[
				(self.raw_ctrl, "Control"),
				(self.alt, "Option"),
				(self.shift, "Shift"),
				(self.ctrl, "Cmd"),
				(self.win, "Win"),
			]
		} else {
			[
				(self.raw_ctrl || self.ctrl, "Ctrl"),
				(self.alt, "Alt"),
				(self.shift, "Shift"),
				(self.win, "Win"),
				(false, ""),
			]
		};
		let mut parts: Vec<&str> = modifiers.iter().filter(|(held, _)| *held).map(|(_, name)| *name).collect();
		parts.push(&self.key);
		parts.join("+")
	}

	/// Parses [`to_shortcut_string`](KeyChord::to_shortcut_string)'s output back.
	///
	/// Returns `None` for an empty string or the literal `"none"`, which is how a config file
	/// spells "this action deliberately has no shortcut".
	#[must_use]
	pub fn parse(input: &str) -> Option<Self> {
		let trimmed = input.trim();
		if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
			return None;
		}
		let mut ctrl = false;
		let mut raw_ctrl = false;
		let mut alt = false;
		let mut shift = false;
		let mut win = false;
		let mut remaining = trimmed;
		while let Some(plus_idx) = remaining.find('+') {
			let prefix = &remaining[..plus_idx];
			if prefix.eq_ignore_ascii_case("rawctrl") {
				raw_ctrl = true;
				remaining = &remaining[plus_idx + 1..];
			} else if prefix.eq_ignore_ascii_case("ctrl") || prefix.eq_ignore_ascii_case("control") {
				ctrl = true;
				remaining = &remaining[plus_idx + 1..];
			} else if prefix.eq_ignore_ascii_case("alt") {
				alt = true;
				remaining = &remaining[plus_idx + 1..];
			} else if prefix.eq_ignore_ascii_case("shift") {
				shift = true;
				remaining = &remaining[plus_idx + 1..];
			} else if prefix.eq_ignore_ascii_case("win") || prefix.eq_ignore_ascii_case("windows") {
				win = true;
				remaining = &remaining[plus_idx + 1..];
			} else {
				break;
			}
		}
		let key = remaining.to_string();
		if key.is_empty() {
			return None;
		}
		let normalized = Self::normalize_key_name(&key);
		Some(Self { ctrl, raw_ctrl, alt, shift, win, key: normalized })
	}

	/// Builds a chord from a live key event.
	///
	/// Returns `None` for a key code with no name here, such as a bare modifier press. That is
	/// what lets a capture field ignore the user reaching for Shift on the way to a chord.
	///
	/// The result never has `win` set, since wx key events don't carry it.
	#[must_use]
	pub fn from_key_code(key_code: i32, ctrl: bool, alt: bool, shift: bool) -> Option<Self> {
		let key_name = if let Some((name, _)) = NAMED_KEYS.iter().find(|(_, codes)| codes.contains(&key_code)) {
			(*name).to_string()
		} else {
			let named = |code: i32| char::from_u32(u32::try_from(code).ok()?).map(|c| c.to_string());
			match key_code {
				WXK_F1..=WXK_F24 => format!("F{}", key_code - WXK_F1 + 1),
				65..=90 | 48..=57 => named(key_code)?,
				97..=122 => named(key_code - 32)?,
				WXK_NUMPAD0..=WXK_NUMPAD9 => named(key_code - WXK_NUMPAD0 + 48)?,
				_ => return None,
			}
		};
		Some(Self { ctrl, raw_ctrl: false, alt, shift, win: false, key: key_name })
	}

	/// Whether a live key event is this chord.
	///
	/// Each key accepts both its ASCII code and its numpad or extended equivalent, so a binding
	/// on `.` fires from the numpad decimal point too.
	///
	/// Always `false` for a `win` chord, which wx key events can't express.
	#[must_use]
	pub fn matches(&self, key_code: i32, ctrl: bool, alt: bool, shift: bool) -> bool {
		if self.win {
			return false;
		}
		let self_ctrl = self.ctrl || self.raw_ctrl;
		if self_ctrl != ctrl || self.alt != alt || self.shift != shift {
			return false;
		}
		let key_str = self.key.as_str();
		if let Some((_, codes)) = NAMED_KEYS.iter().find(|(name, _)| name.eq_ignore_ascii_case(key_str)) {
			return codes.contains(&key_code);
		}
		if key_str.starts_with(['F', 'f'])
			&& let Ok(num) = key_str[1..].parse::<i32>()
			&& (1..=24).contains(&num)
		{
			return key_code == WXK_F1 + num - 1;
		}
		let mut chars = key_str.chars();
		match (chars.next(), chars.next()) {
			(Some(ch), None) if ch.is_ascii_alphabetic() => {
				let upper = ch.to_ascii_uppercase() as i32;
				key_code == upper || key_code == upper + 32
			}
			(Some(ch), None) if ch.is_ascii_digit() => {
				key_code == ch as i32 || key_code == ch as i32 - 48 + WXK_NUMPAD0
			}
			(Some(ch), None) => key_code == ch as i32,
			_ => false,
		}
	}

	/// Whether two chords fire on the same keystroke, and so cannot both be bound.
	///
	/// Not `==`: a `ctrl` chord and a `raw_ctrl` chord on the same key are different values but
	/// the same keystroke everywhere except macOS, so binding both is a conflict.
	#[must_use]
	pub fn conflicts_with(&self, other: &Self) -> bool {
		(self.ctrl || self.raw_ctrl) == (other.ctrl || other.raw_ctrl)
			&& self.alt == other.alt
			&& self.shift == other.shift
			&& self.win == other.win
			&& self.key == other.key
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_and_formats_a_modifier_chord() {
		let chord = KeyChord::parse("Ctrl+Shift+O").unwrap();
		assert!(chord.ctrl);
		assert!(chord.shift);
		assert!(!chord.alt);
		assert!(!chord.raw_ctrl);
		assert_eq!(chord.key, "O");
		assert_eq!(chord.to_shortcut_string(), "Ctrl+Shift+O");
	}

	#[test]
	fn parses_a_bare_key() {
		let single = KeyChord::parse("H").unwrap();
		assert!(!single.ctrl && !single.shift && !single.alt);
		assert_eq!(single.to_shortcut_string(), "H");
	}

	#[test]
	fn an_unbound_action_parses_to_nothing() {
		assert_eq!(KeyChord::parse("none"), None);
		assert_eq!(KeyChord::parse(""), None);
	}

	#[test]
	fn raw_ctrl_survives_a_round_trip() {
		let chord = KeyChord::new_raw_ctrl(true, false, false, "Space");
		assert_eq!(chord.to_shortcut_string(), "RawCtrl+Space");
		assert_eq!(KeyChord::parse("RawCtrl+Space"), Some(chord));
	}

	#[test]
	fn key_names_normalize_to_one_spelling() {
		assert_eq!(KeyChord::normalize_key_name("esc"), "Escape");
		assert_eq!(KeyChord::normalize_key_name("return"), "Enter");
		assert_eq!(KeyChord::normalize_key_name("page up"), "PageUp");
		assert_eq!(KeyChord::normalize_key_name("f5"), "F5");
		assert_eq!(KeyChord::normalize_key_name("o"), "O");
	}

	#[test]
	fn ctrl_and_raw_ctrl_on_one_key_are_a_conflict() {
		// Different values, but the same keystroke everywhere except macOS.
		let cmd = KeyChord::new(true, false, false, "K");
		let control = KeyChord::new_raw_ctrl(true, false, false, "K");
		assert_ne!(cmd, control);
		assert!(cmd.conflicts_with(&control));
	}

	#[test]
	fn a_different_modifier_is_not_a_conflict() {
		let plain = KeyChord::new(true, false, false, "K");
		let shifted = KeyChord::new(true, false, true, "K");
		assert!(!plain.conflicts_with(&shifted));
	}

	#[test]
	fn a_chord_matches_its_own_key_event() {
		let chord = KeyChord::new(true, true, false, "K");
		assert!(chord.matches(75, true, true, false));
		assert!(!chord.matches(75, true, false, false));
	}

	#[test]
	fn a_key_matches_its_numpad_equivalent() {
		let chord = KeyChord::new(false, false, false, ".");
		assert!(chord.matches(46, false, false, false));
		assert!(chord.matches(391, false, false, false));
		assert!(!chord.matches(387, false, false, false));
	}

	#[test]
	fn a_bare_ctrl_press_is_not_ctrl_delete() {
		let chord = KeyChord::new(true, false, false, "Delete");
		assert!(!chord.matches(308, true, false, false));
		assert!(chord.matches(127, true, false, false));
		assert!(chord.matches(385, true, false, false));
		assert_eq!(KeyChord::from_key_code(308, true, false, false), None);
	}

	#[test]
	fn numpad_navigation_keys_match_their_own_keys() {
		for (name, main, numpad) in [
			("Home", 313, 375),
			("Left", 314, 376),
			("Up", 315, 377),
			("Right", 316, 378),
			("Down", 317, 379),
			("PageUp", 366, 380),
			("PageDown", 367, 381),
			("End", 312, 382),
		] {
			let chord = KeyChord::new(false, false, false, name);
			assert!(chord.matches(main, false, false, false), "{name}");
			assert!(chord.matches(numpad, false, false, false), "{name}");
			assert_eq!(KeyChord::from_key_code(numpad, false, false, false), Some(chord), "{name}");
		}
	}

	#[test]
	fn a_raw_ctrl_chord_matches_a_ctrl_key_event() {
		// wx reports the physical Control key as `control_down` on every platform.
		let chord = KeyChord::new_raw_ctrl(true, false, false, "Space");
		assert!(chord.matches(32, true, false, false));
	}

	#[test]
	fn win_survives_a_round_trip() {
		let chord = KeyChord::new(true, false, false, "Up").with_win(true);
		assert_eq!(chord.to_shortcut_string(), "Ctrl+Win+Up");
		assert_eq!(KeyChord::parse(&chord.to_shortcut_string()), Some(chord.clone()));
		assert_eq!(KeyChord::parse("windows+ctrl+up"), Some(chord));
	}

	#[test]
	fn win_is_part_of_the_keystroke() {
		let plain = KeyChord::new(true, false, false, "Up");
		let with_win = plain.clone().with_win(true);
		assert!(!plain.conflicts_with(&with_win));
		assert!(with_win.conflicts_with(&with_win.clone()));
	}

	#[test]
	fn a_win_chord_never_matches_a_key_event() {
		let chord = KeyChord::new(true, false, false, "Up").with_win(true);
		assert!(!chord.matches(315, true, false, false));
	}

	#[test]
	fn a_bare_modifier_press_captures_nothing() {
		assert_eq!(KeyChord::from_key_code(306, false, false, true), None);
	}

	#[test]
	#[cfg(not(target_os = "macos"))]
	fn display_string_reads_both_ctrls_as_ctrl() {
		assert_eq!(KeyChord::new(true, true, true, "k").to_display_string(), "Ctrl+Alt+Shift+K");
		assert_eq!(KeyChord::new_raw_ctrl(true, false, false, "Space").to_display_string(), "Ctrl+Space");
	}

	#[test]
	#[cfg(target_os = "macos")]
	fn display_string_uses_mac_names_and_order() {
		assert_eq!(KeyChord::new(true, true, true, "k").to_display_string(), "Option+Shift+Cmd+K");
		assert_eq!(KeyChord::new_raw_ctrl(true, false, false, "Space").to_display_string(), "Control+Space");
	}
}
