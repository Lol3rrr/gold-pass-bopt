use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Time {
    pub year: usize,
    pub month: usize,
    pub day: usize,
}

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw_value = String::deserialize(deserializer)?;

        if raw_value.len() < 20 {
            return Err(serde::de::Error::custom("".to_string()));
        }

        let raw_year = &raw_value[0..4];
        let raw_month = &raw_value[4..6];
        let raw_day = &raw_value[6..8];

        let year: usize = raw_year.parse().map_err(|e| serde::de::Error::custom(e))?;
        let month: usize = raw_month.parse().map_err(|e| serde::de::Error::custom(e))?;
        let day: usize = raw_day.parse().map_err(|e| serde::de::Error::custom(e))?;

        Ok(Self { year, month, day })
    }
}

impl Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = format!(
            "{:04}{:02}{:02}T000000.000Z",
            self.year, self.month, self.day
        );
        raw.serialize(serializer)
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.year, self.month, self.day).partial_cmp(&(other.year, other.month, other.day))
    }
}
impl Ord for Time {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct TestStruct {
        time: Time,
    }

    #[test]
    fn parse() {
        let content = "{ \"time\": \"20230804T070000.000Z\" }";

        let result: TestStruct = serde_json::from_str(content).unwrap();

        assert_eq!(
            Time {
                year: 2023,
                month: 8,
                day: 4,
            },
            result.time
        );
    }

    #[test]
    fn ordering() {
        let first = Time {
            year: 2023,
            month: 6,
            day: 10,
        };
        let second = Time {
            year: 2023,
            month: 6,
            day: 12,
        };
        let third = Time {
            year: 2023,
            month: 7,
            day: 10,
        };

        assert!(first < second);
        assert!(first < third);
        assert!(second < third);
    }
}
