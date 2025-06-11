use wit_bindgen_core::abi;

fn hexdigit(v: u32) -> char {
    if v < 10 {
        char::from_u32(('0' as u32) + v).unwrap()
    } else {
        char::from_u32(('A' as u32) - 10 + v).unwrap()
    }
}

/// encode symbol as alphanumeric by hex-encoding special characters
pub fn make_external_component(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' => {
                let mut s = String::new();
                s.push(c);
                s
            }
            '-' => {
                let mut s = String::new();
                s.push('_');
                s
            }
            _ => {
                let mut s = String::from("X");
                s.push(hexdigit((c as u32 & 0xf0) >> 4));
                s.push(hexdigit(c as u32 & 0xf));
                s
            }
        })
        .collect()
}

/// encode symbol as alphanumeric by hex-encoding special characters
pub fn make_external_symbol(module_name: &str, name: &str, variant: abi::AbiVariant) -> String {
    if module_name.is_empty() || module_name == "$root" {
        make_external_component(name)
    } else {
        let mut res = make_external_component(module_name);
        res.push_str(if matches!(variant, abi::AbiVariant::GuestExport) {
            "X23" // Hash character
        } else {
            "X00" // NUL character (some tools use '.' for display)
        });
        res.push_str(&make_external_component(name));
        res
    }
}
