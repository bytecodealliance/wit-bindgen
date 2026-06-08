use std::collections::HashSet;

#[derive(Default)]
pub struct Ns {
    defined: HashSet<String>,
    tmp: usize,
}

impl Ns {
    pub fn insert(&mut self, name: &str) -> Result<(), String> {
        if self.defined.insert(name.to_string()) {
            Ok(())
        } else {
            Err(format!("name `{name}` already defined"))
        }
    }

    pub fn tmp(&mut self, name: &str) -> String {
        let mut ret = name.to_string();
        while self.defined.contains(&ret) {
            ret = format!("{}{}", name, self.tmp);
            self.tmp += 1;
        }
        self.defined.insert(ret.clone());
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::Ns;

    #[test]
    fn insert_unique_ok() {
        let mut ns = Ns::default();
        assert!(ns.insert("foo").is_ok());
    }

    #[test]
    fn insert_duplicate_err() {
        let mut ns = Ns::default();
        ns.insert("foo").unwrap();
        assert_eq!(
            ns.insert("foo"),
            Err("name `foo` already defined".to_string())
        );
    }

    #[test]
    fn tmp_returns_base_when_free() {
        let mut ns = Ns::default();
        assert_eq!(ns.tmp("foo"), "foo");
    }

    #[test]
    fn tmp_avoids_inserted_name() {
        let mut ns = Ns::default();
        ns.insert("foo").unwrap();
        assert_eq!(ns.tmp("foo"), "foo0");
    }

    #[test]
    fn tmp_avoids_predefined_suffix() {
        let mut ns = Ns::default();
        ns.insert("foo").unwrap();
        ns.insert("foo0").unwrap();
        assert_eq!(ns.tmp("foo"), "foo1");
    }

    #[test]
    fn tmp_skips_multiple_predefined_suffixes() {
        let mut ns = Ns::default();
        ns.insert("bar").unwrap();
        ns.insert("bar0").unwrap();
        ns.insert("bar1").unwrap();
        assert_eq!(ns.tmp("bar"), "bar2");
    }

    #[test]
    fn tmp_empty_name_edge_case() {
        let mut ns = Ns::default();
        assert_eq!(ns.tmp(""), "");
        assert_eq!(ns.tmp(""), "0");
        assert_eq!(ns.tmp(""), "1");
    }
}
