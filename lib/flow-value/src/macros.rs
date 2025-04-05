#[macro_export]
macro_rules! map {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr_2021),*) => (<[()]>::len(&[$($crate::map!(@single $rest)),*]));

    ($($key:expr_2021 => $value:expr_2021,)+) => { $crate::map!($($key => $value),+) };
    ($($key:expr_2021 => $value:expr_2021),*) => {
        {
            let _cap = $crate::map!(@count $($key),*);
            let mut _map = $crate::Map::with_capacity(_cap);
            $(
                let _ = _map.insert($crate::Key::from($key), $crate::Value::from($value));
            )*
            _map
        }
    };
}

#[macro_export]
macro_rules! array {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr_2021),*) => (<[()]>::len(&[$($crate::array!(@single $rest)),*]));

    ($($value:expr_2021,)+) => { $crate::array!($($value),+) };
    ($($value:expr_2021),*) => {
        {
            let _cap = $crate::array!(@count $($value),*);
            let mut _vec = ::std::vec::Vec::<$crate::Value>::with_capacity(_cap);
            $(
                _vec.push($crate::Value::from($value));
            )*
            _vec
        }
    };
}
