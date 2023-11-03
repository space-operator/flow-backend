use space_lib::space;
use std::f64::consts::PI;

#[space]
fn main(input: f64) -> f64 {
    input * PI
}
