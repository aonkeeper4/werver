// TODO: add support for 4d6kh3, 12d4kl5, etc
// i wanna roll ability scores on this

use std::{fmt::Display, num::ParseIntError, str::FromStr};

use rand::prelude::*;

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum DiceType {
    D4 = 4,
    D6 = 6,
    D8 = 8,
    D10 = 10,
    D12 = 12,
    D20 = 20,
    D100 = 100,
}

impl Display for DiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::D4 => "d4",
            Self::D6 => "d6",
            Self::D8 => "d8",
            Self::D10 => "d10",
            Self::D12 => "d12",
            Self::D20 => "d20",
            Self::D100 => "d100",
        })
    }
}

enum RollProcessing {
    KeepHighest(usize),
    KeepLowest(usize),
    None,
}

pub struct DiceRoll {
    dice_type: DiceType,
    dice_count: usize,
    roll_processing: RollProcessing,
}

impl DiceRoll {
    #[must_use]
    pub fn roll(&self) -> u32 {
        let mut rng = thread_rng();
        let mut rolls: Vec<_> = (0..self.dice_count)
            .map(|_| rng.gen_range(1..=self.dice_type as u32))
            .collect();
        rolls.sort_unstable();
        match self.roll_processing {
            RollProcessing::KeepHighest(n) => {
                rolls.reverse();
                rolls.truncate(n);
            }
            RollProcessing::KeepLowest(n) => rolls.truncate(n),
            RollProcessing::None => (),
        }
        rolls.iter().sum()
    }

    #[must_use]
    pub fn to_english(&self) -> String {
        match self.roll_processing {
            RollProcessing::KeepHighest(keep_count) => format!(
                "{} {}, keeping highest {} rolls",
                self.dice_count, self.dice_type, keep_count
            ),
            RollProcessing::KeepLowest(keep_count) => format!(
                "{} {}, keeping lowest {} rolls",
                self.dice_count, self.dice_type, keep_count
            ),
            RollProcessing::None => format!("{} {}", self.dice_count, self.dice_type),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    // todo
    pub fn prob(&self, res: u32) -> f32 {
        if (res < self.dice_count as u32) || (res > self.dice_count as u32 * self.dice_type as u32)
        {
            0.0
        } else {
            todo!();
        }
    }
}

pub struct ParseDiceRollError(String);

impl From<ParseIntError> for ParseDiceRollError {
    fn from(value: ParseIntError) -> Self {
        Self(format!("{value}"))
    }
}

impl Display for ParseDiceRollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for DiceRoll {
    type Err = ParseDiceRollError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (rest, roll_processing) = if let Some((rest, processing_tokens)) = s.split_once('k') {
            let (low_or_high, count) = processing_tokens.split_at(1);
            let count = count.parse()?;
            (
                rest,
                match low_or_high {
                    "l" => RollProcessing::KeepLowest(count),
                    "h" => RollProcessing::KeepHighest(count),
                    _ => return Err(ParseDiceRollError(format!("Invalid dice string: {s}"))),
                },
            )
        } else {
            (s, RollProcessing::None)
        };
        let Some((dice_count, dice_type)) = rest.split_once('d') else {
            return Err(ParseDiceRollError(format!("Invalid dice string: {s}")));
        };
        let dice_count = if dice_count.is_empty() {
            1
        } else {
            dice_count.parse()?
        };
        let dice_type = match dice_type.to_lowercase().as_str() {
            "4" => DiceType::D4,
            "6" => DiceType::D6,
            "8" => DiceType::D8,
            "10" => DiceType::D10,
            "12" => DiceType::D12,
            "20" => DiceType::D20,
            "100" => DiceType::D100,
            _ => {
                return Err(ParseDiceRollError(format!(
                    "Unknown dice type: {dice_type}"
                )));
            }
        };
        Ok(Self {
            dice_type,
            dice_count,
            roll_processing,
        })
    }
}
