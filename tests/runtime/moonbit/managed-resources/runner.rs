include!(env!("BINDINGS"));

use test::managed_resources::api::*;

struct Component;
export!(Component);

impl Guest for Component {
    fn run() {
        assert_eq!(CheckedCounter::new(0).unwrap_err(), "zero");
        assert_eq!(checked_drop_count(), 0);
        let checked = CheckedCounter::new(42).unwrap();
        assert_eq!(checked.get(), 42);
        drop(checked);
        assert_eq!(checked_drop_count(), 1);

        let first = Counter::new(10);
        let second = Counter::new(20);
        assert_eq!(first.get(), 10);
        first.add(5);
        assert_eq!(read(&first), 15);
        assert_eq!(read_record(&Borrowed { value: &second }), 20);
        let first = bounce(first);
        drop_saved_alias();
        assert_eq!(first.get(), 25);
        assert_eq!(drop_count(), 0);
        consume(first);
        assert_eq!(drop_count(), 1);
        drop(second);
        assert_eq!(drop_count(), 2);

        // Repeated allocation must not confuse reused representations and handles.
        for i in 0..32 {
            let counter = Counter::new(i);
            assert_eq!(counter.get(), i);
            drop(counter);
            assert_eq!(drop_count(), 3 + i);
        }
        drop(Counter::new(999));
        assert_eq!(drop_count(), 36);
    }
}
