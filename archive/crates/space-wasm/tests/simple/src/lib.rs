use space_lib::space;

#[space]
fn main(input: String) -> String {
    input.chars().rev().collect()
}
