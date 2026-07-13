use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cadre {
    #[serde(rename = "JCHEW")]
    Jchew,
    #[serde(rename = "CHEW")]
    Chew,
}

impl Cadre {
    pub const ALL: [Self; 2] = [Self::Jchew, Self::Chew];
}

#[cfg(test)]
mod tests {
    use super::Cadre;

    #[test]
    fn jchew_and_chew_are_the_only_active_cadres() {
        assert_eq!(Cadre::ALL, [Cadre::Jchew, Cadre::Chew]);
    }

    #[test]
    fn cadre_serializes_to_stable_uppercase_codes() {
        assert_eq!(serde_json::to_string(&Cadre::Jchew).unwrap(), "\"JCHEW\"");
        assert_eq!(serde_json::to_string(&Cadre::Chew).unwrap(), "\"CHEW\"");
    }
}
