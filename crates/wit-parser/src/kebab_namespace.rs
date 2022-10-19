use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Index;

/// A namespace for Wit's kebab-case identifiers.
///
/// Wit's kebab-case identifier syntax is defined as:
///
/// ```ebnf
/// name           ::= <word>
///                  | <name>-<word>
/// word           ::= [a-z][0-9a-z]*
///                  | [A-Z][0-9A-Z]*
/// ```
///
/// It allows segments between dashes to either be all lowercase, which is the
/// common case, or all uppercase, indicating an acryonym. This means that Wit
/// kebab-case namespaces must be compared case-insensitively when new names
/// are added, but must also be case-preserving, as the different cases are
/// sometimes significant for consumers such as bindings generators.
///
/// This struct wraps and acts like a `HashMap<String, V>`, adding this
/// case-insensitive and case-preserving behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct KebabNamespace<V: Clone + PartialEq + Hash> {
    /// Map holding lowercased names as keys, and pairs of original name
    /// and value as the values.
    map: HashMap<String, KebabNamed<V>>,
}

/// A struct holding a value of type `V` and its name.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct KebabNamed<V: Clone + PartialEq + Hash> {
    pub name: String,
    pub value: V,
}

impl<V: Clone + PartialEq + Hash> KebabNamespace<V> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Insert a value `v` into the map with name `k`.
    ///
    /// The insertion is done case-insensitively, so if any existing value has
    /// a name which case-insensitively matches `k`, return it.
    pub fn insert(&mut self, k: String, v: V) -> Option<KebabNamed<V>> {
        // When inserting, compare keys case-insensitively.
        self.map
            .insert(k.to_lowercase(), KebabNamed { name: k, value: v })
    }

    /// Get the value with name `k`.
    pub fn get(&self, k: &str) -> Option<&V> {
        self.map.get(&k.to_lowercase()).map(|named| {
            assert_eq!(k, named.name);
            &named.value
        })
    }

    /// Return an iterator over the names and values.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &V)> {
        // Discard `entry.0`, which is the lower-cased name string, as users
        // iterating over the namespace always want the original name.
        self.map.iter().map(|entry| (&entry.1.name, &entry.1.value))
    }
}

impl<V: Clone + PartialEq + Hash> Default for KebabNamespace<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + PartialEq + Hash> Index<&str> for KebabNamespace<V> {
    type Output = V;

    fn index(&self, i: &str) -> &V {
        let named = self.map.index(&i.to_lowercase());
        assert_eq!(i, named.name);
        &named.value
    }
}
