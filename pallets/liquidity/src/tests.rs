use crate::{mock::*, *};

#[test]
fn first_test_case() {
	new_test_ext().execute_with(|| {
		println!("Hello world!");
	});
}
