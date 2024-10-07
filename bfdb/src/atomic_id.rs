#[macro_export]
macro_rules! atomic_id {
    ($name:ident) => {
        paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name(u64);

            impl std::str::FromStr for $name {
                type Err = anyhow::Error;

                fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                    Ok(Self(s.parse::<u64>()?))
                }
            }

            impl serde::Serialize for $name {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                    serializer.serialize_u64(self.0)
                }
            }

            pub struct [<$name Visitor>];

            impl<'de> serde::de::Visitor<'de> for [<$name Visitor>] {
                type Value = $name;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(formatter, "a u64")
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok($name(v))
                }
            }

            impl<'de> serde::Deserialize<'de> for $name {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
                    deserializer.deserialize_u64([<$name Visitor>])
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }

            impl $name {
                pub fn new(db: &sled::Db) -> anyhow::Result<Self> {
                    Ok(Self(db.generate_id()?))
                }

                pub fn inner(&self) -> u64 {
                    self.0
                }
            }
        }
    }
}
