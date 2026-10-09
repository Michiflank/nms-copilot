//! Procedural names for No Man's Sky regions, systems and planets.
//!
//! The game does not store the names of unrenamed regions, systems or planets
//! in the save file; it regenerates them from the address (or planet seed)
//! every time they are shown. This crate reproduces that generator so the
//! copilot can show the same names the game does.
//!
//! Ported from the MIT-licensed Python `nms_namegen`
//! (<https://github.com/hadsh/nms_namegen>, a fork of
//! <https://github.com/stuart/nms_namegen>); see `data/NMS_NAMEGEN_LICENCE`.
//! The port is deliberately literal: every random draw has to happen in the
//! same order as in the game for the names to match.

use std::collections::HashMap;
use std::sync::OnceLock;

const MULTIPLIER: u64 = 0x5A76_F899;
const MASK32: u64 = 0xFFFF_FFFF;
const TINY_DOUBLE: f64 = 2.328_306_437_080_797_4e-10;
const CONST_A: u64 = 0x64DD_8148_2CBD_31D7;
const CONST_B: u64 = 0xE36A_A5C6_1361_2997;
const VOWELS: &[u8] = b"aeiou";

const ALPHASETS_TXT: &str = include_str!("../data/alphasets.txt");
const LETTER_MAP_JSON: &str = include_str!("../data/letter_map.json");

const ROMAN_NUMERALS: [&str; 20] = [
    "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII", "XIII", "XIV", "XV",
    "XVI", "XVII", "XVIII", "XIX", "XX",
];

const REGION_ADORNMENTS: [&str; 20] = [
    "{} Adjunct",
    "{} Void",
    "{} Expanse",
    "{} Terminus",
    "{} Boundary",
    "{} Fringe",
    "{} Cluster",
    "{} Mass",
    "{} Band",
    "{} Cloud",
    "{} Nebula",
    "{} Quadrant",
    "{} Sector",
    "{} Anomaly",
    "{} Conflux",
    "{} Instability",
    "Sea of {}",
    "The Arm of {}",
    "{} Spur",
    "{} Shallows",
];

const PLANET_ADORNMENTS: [&str; 10] = [
    "Prime", "Major", "Minor", "Alpha", "Beta", "Gamma", "Delta", "Omega", "Sigma", "Tau",
];

const PLANET_STYLES: [&str; 11] = [
    "%PROCNORM%",
    "%PROCNORM% %ADORNMENT%",
    "%PROCNORM% %NUMERAL%",
    "%PROCNORM% %SHORTCODE%",
    "%PROCLONG% %PROCSHORT%",
    "%PROCSHORT% %LONGCODE%",
    "%PROCNORM%",
    "%PROCNORM% %ADORNMENT%",
    "%PROCNORM% %NUMERAL%",
    "Style 9",
    "New %PROCNORM%",
];

/// The game's name PRNG: a 32-bit multiply-with-carry generator.
struct Prng {
    seed: u64,
}

impl Prng {
    fn update(&mut self) {
        self.seed = (self.seed & MASK32)
            .wrapping_mul(MULTIPLIER)
            .wrapping_add(self.seed >> 32);
    }

    /// Random integer in `0..range`.
    fn random(&mut self, range: u64) -> u64 {
        self.update();
        ((self.seed & MASK32) * range) >> 32
    }

    fn randi(&mut self) -> u64 {
        self.update();
        self.seed & MASK32
    }
}

/// A node of the letter-weight search tree for one alphaset and first letter.
enum Node {
    /// Branch: descend when the reversed lookup key sorts after `key` reversed.
    Branch { key: Vec<u8>, children: Vec<Node> },
    /// Leaf: next-letter weights for the trigram `key`.
    Leaf {
        key: Vec<u8>,
        weights: Vec<(u8, f64)>,
    },
}

struct Tables {
    alphasets: Vec<&'static [u8]>,
    /// `letter_map[alphaset][first letter]`
    letter_map: Vec<HashMap<u8, Vec<Node>>>,
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let alphasets: Vec<&'static [u8]> = ALPHASETS_TXT.lines().map(str::as_bytes).collect();
        assert_eq!(alphasets.len(), 8, "expected 8 alphasets");

        let json: serde_json::Value =
            serde_json::from_str(LETTER_MAP_JSON).expect("embedded letter map is valid JSON");
        let letter_map = (0..8)
            .map(|i| {
                json[i.to_string()]
                    .as_object()
                    .expect("letter map has every alphaset")
                    .iter()
                    .map(|(letter, nodes)| (letter.as_bytes()[0], parse_nodes(nodes)))
                    .collect()
            })
            .collect();

        Tables {
            alphasets,
            letter_map,
        }
    })
}

fn parse_nodes(value: &serde_json::Value) -> Vec<Node> {
    value
        .as_array()
        .expect("letter map nodes are arrays")
        .iter()
        .map(|node| {
            let key = node[0].as_str().unwrap_or_default().as_bytes().to_vec();
            match node[2].as_str() {
                Some("ja") => Node::Branch {
                    key,
                    children: parse_nodes(&node[1]),
                },
                _ => Node::Leaf {
                    key,
                    weights: node[1]
                        .as_array()
                        .expect("leaf weights are arrays")
                        .iter()
                        .map(|w| {
                            (
                                w["Item1"].as_str().unwrap_or("a").as_bytes()[0],
                                w["Item2"].as_f64().unwrap_or(0.0),
                            )
                        })
                        .collect(),
                },
            }
        })
        .collect()
}

fn reversed(s: &[u8]) -> Vec<u8> {
    s.iter().rev().copied().collect()
}

fn recursive_search<'a>(nodes: &'a [Node], s: &[u8]) -> Option<&'a [(u8, f64)]> {
    for node in nodes {
        match node {
            Node::Branch { key, children } => {
                if reversed(s) > reversed(key)
                    && let Some(found) = recursive_search(children, s)
                {
                    return Some(found);
                }
            }
            Node::Leaf { key, weights } => {
                if s == key.as_slice() {
                    return Some(weights);
                }
            }
        }
    }
    None
}

fn string_weights(s: &[u8], alphaset: u64) -> Option<&'static [(u8, f64)]> {
    let first = *s.first()?;
    let nodes = tables().letter_map.get(alphaset as usize)?.get(&first)?;
    recursive_search(nodes, s)
}

fn chars_from_alphaset(rng: &mut Prng, alphaset: u64) -> &'static [u8] {
    let set = tables().alphasets[(alphaset & 0x07) as usize];
    let r = (rng.random((set.len() / 3) as u64) * 3) as usize;
    &set[r..r + 3]
}

fn is_vowel(c: u8) -> bool {
    VOWELS.contains(&c)
}

fn insert_vowel(name: &mut Vec<u8>, rng: &mut Prng, index: usize) {
    let vowel = VOWELS[rng.random(5) as usize];
    let index = index.min(name.len());
    name.insert(index, vowel);
}

fn consecutive_consonants(name: &[u8]) -> Option<usize> {
    let mut consonance = 0;
    for (i, &c) in name.iter().enumerate() {
        if consonance < 3 {
            if is_vowel(c) {
                consonance = 0;
            } else {
                consonance += 1;
            }
        } else if !b"aeiouy".contains(&c) {
            return Some(i - 3);
        } else {
            consonance = 0;
        }
    }
    None
}

fn generate_name(rng: &mut Prng, mut alphaset: u64, min_length: u64, max_length: u64) -> Vec<u8> {
    let mut name = chars_from_alphaset(rng, alphaset).to_vec();
    let alternate_char_getter = rng.randi() & 0x01 != 0;

    let register = rng.random(max_length - min_length + 1) as i64 + min_length as i64 - 3;
    let mut add = register;

    if register > 0 {
        let mut tries = 8;
        let mut i: i64 = 0;
        while i < register {
            let start = (i as usize).min(name.len());
            let end = (start + 3).min(name.len());
            let weights = string_weights(&name[start..end], alphaset);
            let mut target = rng.randi() as f64 * TINY_DOUBLE;

            match weights {
                None => {
                    tries -= 1;
                    i -= 1;
                    alphaset = (alphaset + 1) & 0x0800_0007;

                    if tries == 0 {
                        if add < 3 {
                            break;
                        }
                        let temp = chars_from_alphaset(rng, alphaset);
                        let prev = name.get((i + 2) as usize).copied().unwrap_or(b'x');
                        if is_vowel(prev) && is_vowel(temp[0]) {
                            name.push(b'\'');
                        } else if !is_vowel(temp[0]) {
                            name.push(VOWELS[(rng.random(5) & 0xFF) as usize]);
                        }
                        name.extend_from_slice(temp);
                        tries = 1;
                        i += 2;
                        add -= 3;
                    }
                }
                Some(weights) => {
                    let index = if alternate_char_getter {
                        target *= (weights.len() - 1) as f64;
                        let sign = if target > 0.0 {
                            1.0
                        } else if target < 0.0 {
                            -1.0
                        } else {
                            0.0
                        };
                        (sign * 0.5 + target) as usize
                    } else {
                        let mut weight = 0.0;
                        let mut j = 0;
                        for (k, &(_, w)) in weights.iter().enumerate() {
                            j = k;
                            weight += w;
                            if weight >= target {
                                break;
                            }
                        }
                        j
                    };
                    name.push(weights[index].0);
                    add -= 1;
                }
            }

            if name.len() > 63 {
                name.truncate(64);
            }
            i += 1;
        }
    }

    // Insert a vowel at the start where needed.
    let (first, second) = (name[0], name[1]);
    if !is_vowel(first) && !is_vowel(second) && (first != b's' || !b"hklmnprtwy".contains(&second))
    {
        let allowed = (second == b'h' && b"ctw".contains(&first))
            || (second == b'l' && b"bcfgps".contains(&first))
            || (second == b'r' && b"bcdfgkpt".contains(&first))
            || (second == b'w' && b"dgt".contains(&first))
            || (second == b'y' && b"hmr".contains(&first));
        if !allowed {
            insert_vowel(&mut name, rng, 1);
        }
    }

    // Insert a vowel at the end where needed.
    if name.len() > 1 {
        let ult = name[name.len() - 1];
        let penult = name[name.len() - 2];
        if (penult != b'g' || is_vowel(ult))
            && ((ult == b'b' && b"gn".contains(&penult))
                || (ult == b'd' && b"bdfghkmpst".contains(&penult))
                || (ult == b'g' && penult == b'l')
                || (ult == b'p' && b"bdhkt".contains(&penult))
                || (ult == b'r' && b"bfg".contains(&penult))
                || (ult == b't' && penult == b'g')
                || (ult == b'w' && !is_vowel(penult)))
        {
            let index = name.len() - 1;
            insert_vowel(&mut name, rng, index);
        }
    }

    // Break up long consonant runs.
    if let Some(index) = consecutive_consonants(&name) {
        let index = index + rng.random(3) as usize + 1;
        insert_vowel(&mut name, rng, index);
    }

    name
}

fn capitalize(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name).to_lowercase();
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn alphaset_for(reg: u64) -> u64 {
    if reg == 0 {
        0x02
    } else if (reg & 0xFF) < 0x04 {
        (reg & 0xFF) + 0x02
    } else {
        0x07
    }
}

/// Seed derivation shared by system and planet names.
fn word_seed(low: u64, high: u64) -> u64 {
    let rol = ((low & 0x0000_FFFF) << 16) | ((low & 0xFFFF_0000) >> 16);
    let rol = (rol ^ low ^ high) & MASK32;
    low.max(1) * MULTIPLIER + rol
}

/// Name of the region containing `portal_code` (`0xPSSSYYZZZXXX`) in `galaxy`.
pub fn region_name(portal_code: u64, galaxy: u8) -> String {
    let galaxy = galaxy as u64;
    let mut register = galaxy >> 1;
    register ^= (galaxy << 32) | (portal_code & MASK32);
    register = register.wrapping_mul(CONST_A);
    register = ((register >> 33) ^ register).wrapping_mul(CONST_B);
    register ^= register >> 33;

    let mut seed_h = (((register & 0xFFFF_0000) >> 16) | ((register & 0x0000_FFFF) << 16))
        ^ (register & MASK32)
        ^ (register >> 32);
    if seed_h == 0 {
        seed_h = 1;
    }
    let mut rng = Prng {
        seed: (register & MASK32) | (seed_h << 32),
    };

    let max_length = rng.random(4) + 6;
    let name = capitalize(&generate_name(&mut rng, 0, 6, max_length));

    if rng.random(0x64) < 0x50 {
        REGION_ADORNMENTS[rng.random(0x14) as usize].replace("{}", &name)
    } else {
        name
    }
}

/// Name of the star system at `portal_code` (`0xPSSSYYZZZXXX`) in `galaxy`.
/// The planet digit is ignored.
pub fn system_name(portal_code: u64, galaxy: u8) -> String {
    let coords = portal_code & MASK32;
    let system_index = ((portal_code & 0x0FFF_0000_0000) >> 24) | galaxy as u64;
    let seed = word_seed(coords, system_index);
    let mut rng = Prng { seed };

    let alphaset = alphaset_for(((seed & MASK32) * 5) >> 32);
    let max_length = rng.random(4) + 6;
    let mut name = capitalize(&generate_name(&mut rng, alphaset, 6, max_length));

    // Hyphenated names.
    if name.len() < 8 && rng.random(4) < 2 {
        let alphaset = alphaset_for(rng.random(5));
        let second = generate_name(&mut rng, alphaset, 3, 5);
        name = format!("{name}-{}", capitalize(&second));
    }

    if rng.random(0x0A) < 0x03 {
        let n = (rng.random(19) + 1).min(19);
        name = format!("{name} {}", ROMAN_NUMERALS[n as usize - 1]);
    }
    name
}

/// Name of the planet with procedural seed `seed` (the first `VP` value of a
/// planet discovery record).
pub fn planet_name(seed: u64) -> String {
    let low = seed & MASK32;
    let high = seed >> 32;
    let mut rng = Prng {
        seed: word_seed(low, high),
    };

    let adornment = ((rng.seed & MASK32) * 10) >> 0x20;
    let code = rng.random(50) + 1;
    let shortcode = (rng.random(0x1A) + 0x41) as u8 as char;
    let numeral = rng.random(0x12) + 2;
    let digit = rng.random(0x09) + 1;
    let alpha = (rng.random(0x1A) + 0x41) as u8 as char;
    let longcode = rng.random(0x59) + 0x0B;

    let procnorm = capitalize(&generate_name(&mut rng, 7, 4, 8));
    let procshort = capitalize(&generate_name(&mut rng, 5, 4, 5));
    let proclong = capitalize(&generate_name(&mut rng, 7, 6, 10));

    let mut style = rng.random(9) as usize;
    let target = rng.randi() as f64 * TINY_DOUBLE;
    if target < 0.035_000_000_1 {
        style = 10;
    }

    PLANET_STYLES[style]
        .replace("%PROCNORM%", &procnorm)
        .replace("%PROCSHORT%", &procshort)
        .replace("%PROCLONG%", &proclong)
        .replace("%ADORNMENT%", PLANET_ADORNMENTS[adornment as usize])
        .replace("%SHORTCODE%", &format!("{shortcode}{}", code % 0x50))
        .replace("%NUMERAL%", ROMAN_NUMERALS[numeral as usize - 1])
        .replace("%LONGCODE%", &format!("{longcode}/{alpha}{digit}"))
}
