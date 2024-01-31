#[macro_export]
macro_rules! hashmap {
    ( $( $k:expr => $v:expr ),* $(,)? ) => {
        {
            let mut m = std::collections::HashMap::new();
            $( m.insert($k, $v); )*
            m
        }
    };
}
pub use hashmap;