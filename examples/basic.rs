use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct BlogPost {
    id: u64,
    title: String,
    body: String,
    #[serde(default)]
    previous_in_series: Option<u64>,
    category: Category,
}

#[derive(Debug, Serialize, Deserialize)]
enum Category {
    Rust,
    Custom(String),
}

fn main() {
    let posts: Vec<BlogPost> =
        rsn::from_str(include_str!("./basic.rsn")).expect("valid rsn in basic.rsn");

    println!("Loaded blog posts: {posts:?}");

    let compact = rsn::to_string(&posts).expect("no errors");
    println!("Compact form:\n{compact}");
    let pretty = rsn::to_string_pretty(&posts).expect("no errors");
    println!("Pretty form:\n{pretty}");
}

#[test]
fn runs() {
    main();
}
