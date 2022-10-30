extern crate proc_macro;
use proc_macro::{TokenStream};

#[proc_macro]
pub fn shape2(body: TokenStream) -> TokenStream {
    let res = body
        .to_string()
        .split_whitespace()
        .map(|row|
             row
             .chars()
             .map(|ch|
                  match ch {
                      'o' => "true",
                      _ => "false",
                  }
             )
             .collect::<Vec<&str>>()
             .join(",") + ";"
        )
        .collect::<Vec<String>>()
        .join("");
    format!("shape![{}]",res).parse().unwrap()
}
