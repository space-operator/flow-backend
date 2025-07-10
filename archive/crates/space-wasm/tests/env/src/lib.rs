use space_lib::space;

#[space]
fn main(input: String) -> String {
    std::env::var(input).unwrap()
}
