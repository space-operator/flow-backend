#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine() {
        let script = r#"
        x += 1;
        x
        "#;
        let mut e = rhai::Engine::new();
        e.register_fn("+", |a: i128, b: i64| a + b as i128);
        let mut scope = rhai::Scope::new();
        scope.push("x", 10i128);
        let res = e
            .eval_with_scope::<rhai::Dynamic>(&mut scope, script)
            .unwrap();
        dbg!(res.try_cast::<i128>().unwrap());
    }
}
