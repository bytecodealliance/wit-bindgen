include!(env!("BINDINGS"));

use exports::Float as Float2;

fn main() {
    let float3 = add(&Float::new(42.0), &Float::new(55.0));
    assert_eq!(float3.get(), 114.0);

    let float3 = Float2::new(22.0);
    assert_eq!(float3.get(), 22. + 1. + 2. + 4. + 3.);

    let res = Float2::add(float3, 7.0);
    assert_eq!(res.get(), 59.0);
}
