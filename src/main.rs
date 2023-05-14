mod db;

fn main() {
    let mode = std::env::args()
        .nth(1)
        .expect("Could not get CLI args");

    println!("{mode}");
}