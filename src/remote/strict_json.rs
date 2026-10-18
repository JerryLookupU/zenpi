//! Preflight JSON before typed decoding can discard duplicate payload keys.

use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::collections::BTreeSet;

const MAX_WIRE_NODES: usize = 8192;
const MAX_WIRE_DEPTH: usize = 40;

struct Seed<'a> {
    nodes: &'a mut usize,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        *self.nodes += 1;
        if *self.nodes > MAX_WIRE_NODES || self.depth > MAX_WIRE_DEPTH {
            return Err(serde::de::Error::custom("remote JSON complexity"));
        }
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("bounded JSON without duplicate members")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(serde::de::Error::custom("duplicate remote JSON member"));
            }
            map.next_value_seed(Seed {
                nodes: self.nodes,
                depth: self.depth + 1,
            })?;
        }
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while seq
            .next_element_seed(Seed {
                nodes: self.nodes,
                depth: self.depth + 1,
            })?
            .is_some()
        {}
        Ok(())
    }

    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }
}

pub(super) fn preflight(bytes: &[u8]) -> Result<(), serde_json::Error> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    Seed {
        nodes: &mut 0,
        depth: 0,
    }
    .deserialize(&mut decoder)?;
    decoder.end()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_node_budget_is_exact_and_overflow_refused() {
        let at_limit = serde_json::to_vec(&vec![0; MAX_WIRE_NODES - 1]).unwrap();
        assert!(preflight(&at_limit).is_ok());
        let over = serde_json::to_vec(&vec![0; MAX_WIRE_NODES]).unwrap();
        assert!(preflight(&over).is_err());
    }

    #[test]
    fn wire_depth_budget_is_exact_and_overflow_refused() {
        let nested = |depth: usize| format!("{}null{}", "[".repeat(depth), "]".repeat(depth));
        assert!(preflight(nested(MAX_WIRE_DEPTH).as_bytes()).is_ok());
        assert!(preflight(nested(MAX_WIRE_DEPTH + 1).as_bytes()).is_err());
        assert!(preflight(b"{} {}").is_err());
        assert!(preflight(&[0xff]).is_err());
    }
}
