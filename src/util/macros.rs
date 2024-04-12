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

#[macro_export]
macro_rules! err_opt_is_none {
    () => {{
        let e = anyhow::anyhow!("Option is None");
        e
    }};
}
