use serde::{Deserialize, Serialize};

#[cfg(feature = "test_utils")]
use arbitrary::{self, Arbitrary};

// Internal constants
const OUTPUT_BUILTIN_NAME: &str = "output";
const HASH_BUILTIN_NAME: &str = "pedersen";
const RANGE_CHECK_BUILTIN_NAME: &str = "range_check";
const RANGE_CHECK_96_BUILTIN_NAME: &str = "range_check96";
const SIGNATURE_BUILTIN_NAME: &str = "ecdsa";
const BITWISE_BUILTIN_NAME: &str = "bitwise";
const EC_OP_BUILTIN_NAME: &str = "ec_op";
const KECCAK_BUILTIN_NAME: &str = "keccak";
const POSEIDON_BUILTIN_NAME: &str = "poseidon";
const SEGMENT_ARENA_BUILTIN_NAME: &str = "segment_arena";
const ADD_MOD_BUILTIN_NAME: &str = "add_mod";
const MUL_MOD_BUILTIN_NAME: &str = "mul_mod";

const OUTPUT_BUILTIN_NAME_WITH_SUFFIX: &str = "output_builtin";
const HASH_BUILTIN_NAME_WITH_SUFFIX: &str = "pedersen_builtin";
const RANGE_CHECK_BUILTIN_NAME_WITH_SUFFIX: &str = "range_check_builtin";
const RANGE_CHECK_96_BUILTIN_NAME_WITH_SUFFIX: &str = "range_check96_builtin";
const SIGNATURE_BUILTIN_NAME_WITH_SUFFIX: &str = "ecdsa_builtin";
const BITWISE_BUILTIN_NAME_WITH_SUFFIX: &str = "bitwise_builtin";
const EC_OP_BUILTIN_NAME_WITH_SUFFIX: &str = "ec_op_builtin";
const KECCAK_BUILTIN_NAME_WITH_SUFFIX: &str = "keccak_builtin";
const POSEIDON_BUILTIN_NAME_WITH_SUFFIX: &str = "poseidon_builtin";
const SEGMENT_ARENA_BUILTIN_NAME_WITH_SUFFIX: &str = "segment_arena_builtin";
const ADD_MOD_BUILTIN_NAME_WITH_SUFFIX: &str = "add_mod_builtin";
const MUL_MOD_BUILTIN_NAME_WITH_SUFFIX: &str = "mul_mod_builtin";

/// Enum representing the name of a cairo builtin
#[cfg_attr(feature = "test_utils", derive(Arbitrary))]
#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Eq, Hash, Ord, PartialOrd)]
#[allow(non_camel_case_types)]
pub enum BuiltinName {
    output,
    range_check,
    pedersen,
    ecdsa,
    keccak,
    bitwise,
    ec_op,
    poseidon,
    segment_arena,
    range_check96,
    add_mod,
    mul_mod,
}

impl BuiltinName {
    /// Converts a [`BuiltinName`] to its string representation adding the "_builtin" suffix
    ///
    /// ## Example
    ///
    /// ```
    /// # use cairo_vm::types::builtin_name::BuiltinName;
    ///
    /// let builtin_name = BuiltinName::poseidon;
    /// assert_eq!(builtin_name.to_str_with_suffix(), "poseidon_builtin");
    ///
    /// ```
    pub fn to_str_with_suffix(self) -> &'static str {
        match self {
            BuiltinName::output => OUTPUT_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::range_check => RANGE_CHECK_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::pedersen => HASH_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::ecdsa => SIGNATURE_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::keccak => KECCAK_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::bitwise => BITWISE_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::ec_op => EC_OP_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::poseidon => POSEIDON_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::segment_arena => SEGMENT_ARENA_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::range_check96 => RANGE_CHECK_96_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::add_mod => ADD_MOD_BUILTIN_NAME_WITH_SUFFIX,
            BuiltinName::mul_mod => MUL_MOD_BUILTIN_NAME_WITH_SUFFIX,
        }
    }

    /// Converts a [`BuiltinName`] to its string representation
    ///
    /// ## Example
    ///
    /// ```
    /// # use cairo_vm::types::builtin_name::BuiltinName;
    ///
    /// let builtin_name = BuiltinName::poseidon;
    /// assert_eq!(builtin_name.to_str(), "poseidon");
    ///
    /// ```
    pub fn to_str(self) -> &'static str {
        match self {
            BuiltinName::output => OUTPUT_BUILTIN_NAME,
            BuiltinName::range_check => RANGE_CHECK_BUILTIN_NAME,
            BuiltinName::pedersen => HASH_BUILTIN_NAME,
            BuiltinName::ecdsa => SIGNATURE_BUILTIN_NAME,
            BuiltinName::keccak => KECCAK_BUILTIN_NAME,
            BuiltinName::bitwise => BITWISE_BUILTIN_NAME,
            BuiltinName::ec_op => EC_OP_BUILTIN_NAME,
            BuiltinName::poseidon => POSEIDON_BUILTIN_NAME,
            BuiltinName::segment_arena => SEGMENT_ARENA_BUILTIN_NAME,
            BuiltinName::range_check96 => RANGE_CHECK_96_BUILTIN_NAME,
            BuiltinName::add_mod => ADD_MOD_BUILTIN_NAME,
            BuiltinName::mul_mod => MUL_MOD_BUILTIN_NAME,
        }
    }

    /// Converts a [`BuiltinName`] from its string representation removing the "_builtin" suffix
    ///
    /// ## Example
    ///
    /// ```
    /// # use cairo_vm::types::builtin_name::BuiltinName;
    ///
    /// assert_eq!(BuiltinName::from_str_with_suffix("poseidon_builtin"), Some(BuiltinName::poseidon));
    ///
    /// assert_eq!(BuiltinName::from_str_with_suffix("unknown"), None);
    ///
    /// ```
    pub fn from_str_with_suffix(suffixed_str: &str) -> Option<Self> {
        match suffixed_str {
            OUTPUT_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::output),
            RANGE_CHECK_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::range_check),
            HASH_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::pedersen),
            SIGNATURE_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::ecdsa),
            KECCAK_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::keccak),
            BITWISE_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::bitwise),
            EC_OP_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::ec_op),
            POSEIDON_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::poseidon),
            SEGMENT_ARENA_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::segment_arena),
            RANGE_CHECK_96_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::range_check96),
            ADD_MOD_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::add_mod),
            MUL_MOD_BUILTIN_NAME_WITH_SUFFIX => Some(BuiltinName::mul_mod),
            _ => None,
        }
    }

    // Implementing this as a trait would generate confusion as `Display` impl uses suffixed version
    #[allow(clippy::should_implement_trait)]
    /// Converts a [`BuiltinName`] from its string representation
    ///
    /// ## Example
    ///
    /// ```
    /// # use cairo_vm::types::builtin_name::BuiltinName;
    ///
    /// assert_eq!(BuiltinName::from_str("poseidon"), Some(BuiltinName::poseidon));
    ///
    /// assert_eq!(BuiltinName::from_str("unknown"), None);
    ///
    /// ```
    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            OUTPUT_BUILTIN_NAME => Some(BuiltinName::output),
            RANGE_CHECK_BUILTIN_NAME => Some(BuiltinName::range_check),
            HASH_BUILTIN_NAME => Some(BuiltinName::pedersen),
            SIGNATURE_BUILTIN_NAME => Some(BuiltinName::ecdsa),
            KECCAK_BUILTIN_NAME => Some(BuiltinName::keccak),
            BITWISE_BUILTIN_NAME => Some(BuiltinName::bitwise),
            EC_OP_BUILTIN_NAME => Some(BuiltinName::ec_op),
            POSEIDON_BUILTIN_NAME => Some(BuiltinName::poseidon),
            SEGMENT_ARENA_BUILTIN_NAME => Some(BuiltinName::segment_arena),
            RANGE_CHECK_96_BUILTIN_NAME => Some(BuiltinName::range_check96),
            ADD_MOD_BUILTIN_NAME => Some(BuiltinName::add_mod),
            MUL_MOD_BUILTIN_NAME => Some(BuiltinName::mul_mod),
            _ => None,
        }
    }
}

/// NOTE: Adds "_builtin" suffix
impl core::fmt::Display for BuiltinName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.to_str_with_suffix().fmt(f)
    }
}

// Implementation of custom serialization & deserialization for maps using builtin names with suffixes as keys
pub(crate) mod serde_generic_map_impl {
    use super::BuiltinName;
    use crate::stdlib::{collections::BTreeMap, string::String};
    use serde::{de::Error, ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, V>(
        values: &BTreeMap<BuiltinName, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let mut map_serializer = serializer.serialize_map(Some(values.len()))?;
        for (key, val) in values {
            map_serializer.serialize_entry(key.to_str_with_suffix(), val)?
        }
        map_serializer.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>, V: Deserialize<'de>>(
        d: D,
    ) -> Result<BTreeMap<BuiltinName, V>, D::Error> {
        // First deserialize keys into String
        let map = BTreeMap::<String, V>::deserialize(d)?;
        // Then match keys to BuiltinName and handle invalid names
        map.into_iter()
            .map(|(k, v)| BuiltinName::from_str_with_suffix(&k).map(|k| (k, v)))
            .collect::<Option<BTreeMap<_, _>>>()
            .ok_or(D::Error::custom("Invalid builtin name"))
    }
}
